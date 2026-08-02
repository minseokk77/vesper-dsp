package com.crossdrop.contentaccess

import android.Manifest
import android.app.Activity
import android.content.ClipData
import android.content.ContentValues
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.os.ParcelFileDescriptor
import android.provider.DocumentsContract
import android.provider.MediaStore
import android.provider.OpenableColumns
import app.tauri.annotation.Command
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.PermissionState
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import androidx.activity.result.ActivityResult
import com.google.zxing.integration.android.IntentIntegrator
import java.net.URLConnection
import java.io.File
import java.io.FileOutputStream
import java.util.concurrent.ConcurrentHashMap
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream

@InvokeArg
class OpenUriArgs {
    lateinit var uri: String
}

@InvokeArg
class CreateReceivedFileArgs {
    lateinit var fileName: String
}

@InvokeArg
class FinishReceivedFileArgs {
    lateinit var uri: String
    var success: Boolean = false
}

private const val LOCAL_NETWORK_PERMISSION = "localNetwork"

@TauriPlugin(
    permissions = [
        Permission(
            strings = [Manifest.permission.NEARBY_WIFI_DEVICES],
            alias = LOCAL_NETWORK_PERMISSION,
        ),
    ],
)
class ContentAccessPlugin(private val activity: Activity) : Plugin(activity) {
    private val pendingReceivedFiles = ConcurrentHashMap.newKeySet<String>()

    @Command
    fun deviceName(invoke: Invoke) {
        invoke.resolve(JSObject().apply { put("name", Build.MODEL) })
    }

    @Command
    fun requestLocalNetworkAccess(invoke: Invoke) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) {
            invoke.resolve()
            return
        }
        if (getPermissionState(LOCAL_NETWORK_PERMISSION) == PermissionState.GRANTED) {
            invoke.resolve()
        } else {
            requestPermissionForAlias(
                LOCAL_NETWORK_PERMISSION,
                invoke,
                "localNetworkPermissionCallback",
            )
        }
    }

    @PermissionCallback
    private fun localNetworkPermissionCallback(invoke: Invoke) {
        if (getPermissionState(LOCAL_NETWORK_PERMISSION) == PermissionState.GRANTED) {
            invoke.resolve()
        } else {
            invoke.reject("주변 기기 권한이 필요합니다. 앱 설정에서 허용해 주세요.")
        }
    }

    @Command
    fun openUri(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(OpenUriArgs::class.java)
            val uri = Uri.parse(args.uri)
            if (uri.scheme != "content") {
                invoke.reject("선택한 Android 파일 URI가 올바르지 않습니다.")
                return
            }

            if (DocumentsContract.isTreeUri(uri) || activity.contentResolver.getType(uri) == DocumentsContract.Document.MIME_TYPE_DIR) {
                resolveArchivedTree(invoke, uri)
                return
            }
            val metadata = queryMetadata(uri)
            val descriptor = activity.contentResolver.openFileDescriptor(uri, "r")
                ?: throw IllegalStateException("선택한 파일을 열 수 없습니다.")
            val size = when {
                descriptor.statSize >= 0 -> descriptor.statSize
                metadata.second >= 0 -> metadata.second
                else -> {
                    descriptor.close()
                    throw IllegalStateException("선택한 파일 크기를 확인할 수 없습니다.")
                }
            }
            val fd = descriptor.detachFd()
            invoke.resolve(JSObject().apply {
                put("fd", fd)
                put("fileName", metadata.first)
                put("fileSize", size)
            })
        } catch (error: Exception) {
            invoke.reject(error.message ?: "선택한 파일을 읽을 수 없습니다.")
        }
    }

    @Command
    fun pickFolder(invoke: Invoke) {
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT_TREE).apply {
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            addFlags(Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION)
        }
        startActivityForResult(invoke, intent, "pickFolderResult")
    }

    @ActivityCallback
    fun pickFolderResult(invoke: Invoke, result: ActivityResult) {
        val uri = result.data?.data
        if (result.resultCode != Activity.RESULT_OK || uri == null) {
            invoke.reject("폴더 선택을 취소했습니다.")
            return
        }
        try {
            activity.contentResolver.takePersistableUriPermission(
                uri,
                Intent.FLAG_GRANT_READ_URI_PERMISSION,
            )
        } catch (_: SecurityException) {}
        invoke.resolve(JSObject().apply { put("uri", uri.toString()) })
    }

    @Command
    fun startBackgroundService(invoke: Invoke) {
        try {
            val intent = Intent(activity, CrossDropForegroundService::class.java)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                activity.startForegroundService(intent)
            } else {
                activity.startService(intent)
            }
            invoke.resolve()
        } catch (error: Exception) {
            invoke.reject(error.message ?: "백그라운드 수신을 시작할 수 없습니다.")
        }
    }

    @Command
    fun stopBackgroundService(invoke: Invoke) {
        try {
            activity.stopService(Intent(activity, CrossDropForegroundService::class.java))
            invoke.resolve()
        } catch (error: Exception) {
            invoke.reject(error.message ?: "백그라운드 수신을 중지할 수 없습니다.")
        }
    }

    @Command
    fun scanPairingQr(invoke: Invoke) {
        try {
            val intent = IntentIntegrator(activity)
                .setDesiredBarcodeFormats(IntentIntegrator.QR_CODE)
                .setPrompt("컴퓨터의 Vesper Drop QR 코드를 비추세요")
                .setBeepEnabled(false)
                .setOrientationLocked(true)
                .setCaptureActivity(PortraitCaptureActivity::class.java)
                .createScanIntent()
            startActivityForResult(invoke, intent, "scanPairingQrResult")
        } catch (error: Exception) {
            invoke.reject(error.message ?: "QR 스캐너를 열 수 없습니다.")
        }
    }

    @ActivityCallback
    fun scanPairingQrResult(invoke: Invoke, result: ActivityResult) {
        val scan = IntentIntegrator.parseActivityResult(result.resultCode, result.data)
        val contents = scan?.contents
        if (contents.isNullOrBlank()) {
            invoke.reject("QR 스캔을 취소했습니다.")
        } else {
            invoke.resolve(JSObject().apply { put("data", contents) })
        }
    }

    @Command
    fun createReceivedFile(invoke: Invoke) {
        try {
            if (Build.VERSION.SDK_INT < Build.VERSION_CODES.Q) {
                invoke.reject("Android 10 이상에서 공개 다운로드 폴더 저장을 지원합니다.")
                return
            }

            val args = invoke.parseArgs(CreateReceivedFileArgs::class.java)
            val values = ContentValues().apply {
                put(MediaStore.MediaColumns.DISPLAY_NAME, args.fileName)
                put(
                    MediaStore.MediaColumns.MIME_TYPE,
                    URLConnection.guessContentTypeFromName(args.fileName)
                        ?: "application/octet-stream",
                )
                put(
                    MediaStore.MediaColumns.RELATIVE_PATH,
                    "${Environment.DIRECTORY_DOWNLOADS}/Vesper Drop",
                )
                put(MediaStore.MediaColumns.IS_PENDING, 1)
            }
            val resolver = activity.contentResolver
            val uri = resolver.insert(MediaStore.Downloads.EXTERNAL_CONTENT_URI, values)
                ?: throw IllegalStateException("다운로드 폴더에 파일을 만들 수 없습니다.")
            val descriptor = try {
                resolver.openFileDescriptor(uri, "w")
                    ?: throw IllegalStateException("수신 파일을 열 수 없습니다.")
            } catch (error: Exception) {
                resolver.delete(uri, null, null)
                throw error
            }
            val fd = descriptor.detachFd()
            pendingReceivedFiles.add(uri.toString())
            invoke.resolve(JSObject().apply {
                put("fd", fd)
                put("uri", uri.toString())
                put("fileName", args.fileName)
            })
        } catch (error: Exception) {
            invoke.reject(error.message ?: "수신 파일을 만들 수 없습니다.")
        }
    }

    @Command
    fun finishReceivedFile(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(FinishReceivedFileArgs::class.java)
            if (!pendingReceivedFiles.remove(args.uri)) {
                invoke.reject("현재 수신 중인 Vesper Drop 파일이 아닙니다.")
                return
            }

            val uri = Uri.parse(args.uri)
            val resolver = activity.contentResolver
            if (args.success) {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                    resolver.update(
                        uri,
                        ContentValues().apply { put(MediaStore.MediaColumns.IS_PENDING, 0) },
                        null,
                        null,
                    )
                }
            } else {
                resolver.delete(uri, null, null)
            }
            invoke.resolve()
        } catch (error: Exception) {
            invoke.reject(error.message ?: "수신 파일을 마무리할 수 없습니다.")
        }
    }

    @Command
    fun openReceivedFolder(invoke: Invoke) {
        val documentUri = DocumentsContract.buildDocumentUri(
            "com.android.externalstorage.documents",
            "primary:${Environment.DIRECTORY_DOWNLOADS}/Vesper Drop",
        )
        activity.runOnUiThread {
            try {
                val viewIntent = Intent(Intent.ACTION_VIEW).apply {
                    setDataAndType(documentUri, DocumentsContract.Document.MIME_TYPE_DIR)
                    addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                }
                activity.startActivity(viewIntent)
                invoke.resolve()
            } catch (_: Exception) {
                try {
                    val pickerIntent = Intent(Intent.ACTION_OPEN_DOCUMENT_TREE).apply {
                        putExtra(DocumentsContract.EXTRA_INITIAL_URI, documentUri)
                    }
                    activity.startActivity(pickerIntent)
                    invoke.resolve()
                } catch (error: Exception) {
                    invoke.reject(error.message ?: "수신 폴더를 열 앱을 찾을 수 없습니다.")
                }
            }
        }
    }

    private fun queryMetadata(uri: Uri): Pair<String, Long> {
        var name = "file"
        var size = -1L
        activity.contentResolver.query(
            uri,
            arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE),
            null,
            null,
            null,
        )?.use { cursor ->
            if (cursor.moveToFirst()) {
                val nameIndex = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                val sizeIndex = cursor.getColumnIndex(OpenableColumns.SIZE)
                if (nameIndex >= 0 && !cursor.isNull(nameIndex)) name = cursor.getString(nameIndex)
                if (sizeIndex >= 0 && !cursor.isNull(sizeIndex)) size = cursor.getLong(sizeIndex)
            }
        }
        return Pair(name, size)
    }

    private fun resolveArchivedTree(invoke: Invoke, treeUri: Uri) {
        val rootId = DocumentsContract.getTreeDocumentId(treeUri)
        val rootUri = DocumentsContract.buildDocumentUriUsingTree(treeUri, rootId)
        val rootName = queryDocumentName(rootUri).ifBlank { "folder" }
        val archive = File.createTempFile("cross-drop-folder-", ".zip", activity.cacheDir)
        ZipOutputStream(FileOutputStream(archive)).use { zip ->
            appendDocumentTree(zip, treeUri, rootId, rootName)
        }
        resolveTemporaryFile(invoke, archive, "$rootName.zip")
    }

    private fun appendDocumentTree(
        zip: ZipOutputStream,
        treeUri: Uri,
        documentId: String,
        relativePath: String,
    ) {
        val children = DocumentsContract.buildChildDocumentsUriUsingTree(treeUri, documentId)
        activity.contentResolver.query(
            children,
            arrayOf(
                DocumentsContract.Document.COLUMN_DOCUMENT_ID,
                DocumentsContract.Document.COLUMN_DISPLAY_NAME,
                DocumentsContract.Document.COLUMN_MIME_TYPE,
            ),
            null,
            null,
            null,
        )?.use { cursor ->
            val idIndex = cursor.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_DOCUMENT_ID)
            val nameIndex = cursor.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_DISPLAY_NAME)
            val mimeIndex = cursor.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_MIME_TYPE)
            while (cursor.moveToNext()) {
                val childId = cursor.getString(idIndex)
                val name = cursor.getString(nameIndex).replace('/', '_')
                val mime = cursor.getString(mimeIndex)
                val childUri = DocumentsContract.buildDocumentUriUsingTree(treeUri, childId)
                val path = "$relativePath/$name"
                if (mime == DocumentsContract.Document.MIME_TYPE_DIR) {
                    zip.putNextEntry(ZipEntry("$path/"))
                    zip.closeEntry()
                    appendDocumentTree(zip, treeUri, childId, path)
                } else {
                    zip.putNextEntry(ZipEntry(path))
                    activity.contentResolver.openInputStream(childUri)?.use { input ->
                        input.copyTo(zip, 128 * 1024)
                    } ?: throw IllegalStateException("폴더 안의 $name 파일을 읽을 수 없습니다.")
                    zip.closeEntry()
                }
            }
        }
    }

    private fun queryDocumentName(uri: Uri): String {
        var name = "folder"
        activity.contentResolver.query(
            uri,
            arrayOf(DocumentsContract.Document.COLUMN_DISPLAY_NAME),
            null,
            null,
            null,
        )?.use { cursor ->
            if (cursor.moveToFirst()) name = cursor.getString(0)
        }
        return name
    }

    private fun resolveTemporaryFile(invoke: Invoke, file: File, fileName: String) {
        val size = file.length()
        val descriptor = ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY)
        val fd = descriptor.detachFd()
        file.delete()
        invoke.resolve(JSObject().apply {
            put("fd", fd)
            put("fileName", fileName)
            put("fileSize", size)
        })
    }
}
