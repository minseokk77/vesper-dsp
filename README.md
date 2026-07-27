# 🎧 Vesper (Private Source Monorepo)

**Vesper**는 오디오 애호가와 전문가를 위해 설계된 고품질 오디오 소프트웨어 생태계입니다.
본 저장소는 **Vesper DSP**와 **Vesper Woofer** 등 Vesper 프로젝트의 전체 소스코드를 통합 관리하는 비공개(Private) 모노레포(Monorepo)입니다.

⚠️ **이 저장소는 Vesper 프로젝트의 핵심 소스코드를 포함하는 비공개 저장소입니다.**
각 앱의 배포용 설치 파일 및 자동 업데이트 서버는 별도의 공개 저장소에서 관리됩니다:
- **Vesper DSP 배포 저장소:** [minseokk77/vesper-dsp](https://github.com/minseokk77/vesper-dsp)
- **Vesper Woofer 배포 저장소:** (해당 공개 저장소 링크)

## 📦 포함된 프로젝트 (Projects)

### 1. Vesper DSP (`/dsp`)
시스템 와이드 디지털 시그널 프로세서(DSP) 및 오디오 엔진입니다.
- **주요 기능:** 가상 오디오 케이블(VAC) 라우팅, 실시간 리샘플링, AutoEQ 및 Spinorama 주파수 보정 연동, 윈도우 시스템 트레이 구동.
- **기술 스택:** Tauri v2, Rust (cpal, rubato), SvelteKit, TailwindCSS.

### 2. Vesper Woofer (`/woofer`)
Vesper DSP와 완벽한 시너지를 발휘하는 **서브우퍼 및 베이스(Bass) 채널 전용 딜레이/크로스오버 동기화 유틸리티**입니다.
- **주요 기능:** 메인 스피커(DSP)와의 미세한 위상차/시간차 밀리초 단위 정밀 보정(Delay Sync), 맞춤형 컷오프(Cut-off) 필터 지원.
- **기술 스택:** Tauri v2, Rust, SvelteKit.

## 🚀 로컬 개발 및 빌드 (Development & Build)

### 사전 요구사항 (Prerequisites)
- Node.js (v18+)
- pnpm
- Rust (최신 안정화 버전)
- Tauri v2 개발 환경 (C++ Build Tools 등)

### Vesper DSP 개발 서버 실행
```bash
cd dsp
pnpm install
pnpm tauri dev
```

### Vesper DSP 릴리즈 빌드
배포용 Windows 설치 파일(`.exe` / `.msi`)을 생성합니다.
```bash
cd dsp
pnpm tauri build
```
*(빌드 후 배포 시에는 `pnpm tauri signer sign` 명령을 통해 `.sig` 보안 서명을 생성한 뒤 공개 저장소의 `updater.json`에 갱신해야 합니다.)*

## 📜 오픈소스 고지 및 라이선스 (Credits & License)

- **AutoEq:** Copyright (c) 2018 Jaakko Pasanen (MIT License)
- **Spinorama:** Copyright (c) 2020 Pierre Aubert (MIT License)

### Proprietary License
**Copyright (c) 2026 Vesper (minseokk77). All Rights Reserved.**
본 저장소의 모든 소스코드와 지적재산권은 원작자에게 귀속되며, 허가 없는 무단 복제, 배포, 리버스 엔지니어링 및 상업적 이용을 엄격히 금지합니다.
