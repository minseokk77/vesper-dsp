# Vesper Drop

Windows와 Android 사이에서 클라우드 없이 파일을 직접 전송하는 LAN 전용 Tauri 2 앱입니다.

## 주요 기능

- UDP 멀티캐스트와 LAN 브로드캐스트 기반 자동 기기 탐색
- 6자리 숫자 코드 또는 QR 데이터 기반 상호 신뢰 기기 페어링
- 처음 보는 기기의 수신 승인, 신뢰 기기의 자동 수신
- SHA-256 체크섬과 수신 저장 ACK 이후에만 송신 완료 처리
- 연결·승인·유휴 타임아웃, 최대 100 GiB 제한, 송수신 각 1개 직렬 처리
- 파일 여러 개, 폴더 ZIP 전송, 취소, 재시도, 전송 기록
- 드래그 앤 드롭 전송
- 정확한 현재·전체 평균·구간 최고 속도와 ETA 표시
- Windows 로그인 자동 시작, 단일 실행, 트레이 백그라운드 수신
- Android foreground service 기반 백그라운드 수신
- Windows 수신 폴더 변경 및 기기 이름/신뢰 기기 관리

수신 파일은 Windows의 선택한 폴더(기본 `다운로드/Vesper Drop`) 또는 Android 공개 `Download/Vesper Drop`에 저장됩니다. 폴더 전송은 `<폴더 이름>.zip`으로 묶어서 전송합니다.

## 보안 범위

현재 프로토콜은 같은 LAN 안에서만 동작합니다. 숫자 코드와 QR은 LAN 기기 신뢰 등록용이며 인터넷 릴레이나 외부망 연결에는 사용하지 않습니다. 파일 내용은 SHA-256으로 검증하지만 아직 전송 구간 암호화는 제공하지 않습니다. 공용 Wi-Fi에서는 사용하지 않는 것을 권장합니다.

## 개발 및 검증

```powershell
pnpm install
pnpm check
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tauri build --debug --no-bundle
```

Android 디버그 APK:

```powershell
$env:ANDROID_HOME = "$env:LOCALAPPDATA\Android\Sdk"
$env:NDK_HOME = "$env:ANDROID_HOME\ndk\27.0.12077973"
$env:JAVA_HOME = "C:\Program Files\Java\jdk-21"
pnpm tauri android build --debug --target aarch64
```

생성 위치: `src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`
