$ErrorActionPreference = 'Stop'

$driverRoot = Join-Path $PSScriptRoot 'vendor\simpleaudiosample'
$mainRoot = Join-Path $driverRoot 'Source\Main'
$packageProject = Join-Path $driverRoot 'Package\package.VcxProj'
$utilitiesProject = Join-Path $driverRoot 'Source\Utilities\Utilities.vcxProj'
$filtersProject = Join-Path $driverRoot 'Source\Filters\Filters.vcxProj'
$wdkRoot = 'C:\Program Files (x86)\Windows Kits\10'
$wdkContentRoot = $wdkRoot + '\'
$msbuild = 'C:\Program Files\Microsoft Visual Studio\18\Community\MSBuild\Current\Bin\amd64\MSBuild.exe'
$inf2cat = Join-Path $wdkRoot 'bin\10.0.26100.0\x86\Inf2Cat.exe'
$signTool = Join-Path $wdkRoot 'bin\10.0.26100.0\x64\signtool.exe'
$certificateThumbprint = '84388A26D55726C9049D5B730AB0D4DF9112F1AC'
$env:WDKContentRoot = $wdkContentRoot

if (-not (Test-Path $msbuild)) {
    throw "MSBuild was not found: $msbuild"
}

if (-not (Test-Path $wdkRoot)) {
    throw "WDK was not found: $wdkRoot"
}

if (-not (Test-Path $inf2cat) -or -not (Test-Path $signTool)) {
    throw 'WDK signing tools were not found.'
}

Copy-Item (Join-Path $mainRoot 'VesperVirtualAudio.inx.template') (Join-Path $mainRoot 'VesperVirtualAudio.inx') -Force

foreach ($project in @($utilitiesProject, $filtersProject, $packageProject)) {
    & $msbuild $project `
        /t:Rebuild `
        /p:Configuration=Release `
        /p:Platform=x64 `
        /p:WDKBuildFolder=10.0.26100.0 `
        /p:VisualStudioVersion=17.0 `
        /p:SkipPackageVerification=true `
        /p:ApiValidator_Enable=false `
        /m:1

    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

$outputRoot = Join-Path $driverRoot 'Package\x64\Release\package'
$catalog = Join-Path $outputRoot 'vespervirtualaudio.cat'

& $inf2cat /driver:$outputRoot /os:10_X64
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

& $signTool sign /fd SHA256 /sha1 $certificateThumbprint /s My $catalog
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

Get-Item (Join-Path $outputRoot 'VesperVirtualAudio.sys'), (Join-Path $outputRoot 'VesperVirtualAudio.inf'), $catalog
