# 🎧 Vesper (Private Source Monorepo)

**Vesper**는 오디오 및 데스크톱 인프라를 위해 설계된 고품질 소프트웨어 생태계입니다.
본 저장소는 **Vesper DSP**, **Vesper Woofer**, **Vesper Gate** 등 Vesper 프로젝트의 전체 소스코드를 통합 관리하는 비공개(Private) 모노레포(Monorepo)입니다.

⚠️ **이 저장소는 Vesper 프로젝트의 핵심 소스코드를 포함하는 비공개 저장소입니다.**
각 앱의 배포용 실행/설치 파일은 릴리즈 에셋에서 관리됩니다:
- **Vesper DSP 배포 저장소:** [minseokk77/vesper-dsp](https://github.com/minseokk77/vesper-dsp)
- **Vesper Gate 배포 저장소:** [minseokk77/vesper-gate](https://github.com/minseokk77/vesper-gate)

## 📦 포함된 프로젝트 (Projects)

### 1. Vesper DSP (`/dsp`)
시스템 와이드 디지털 시그널 프로세서(DSP) 및 오디오 엔진입니다.
- **주요 기능:** 가상 오디오 케이블(VAC) 라우팅, 실시간 리샘플링, AutoEQ 및 Spinorama 주파수 보정 연동, 윈도우 시스템 트레이 구동.
- **기술 스택:** Tauri v2, Rust (cpal, rubato), SvelteKit, TailwindCSS.

### 2. Vesper Woofer (`/woofer`)
서브우퍼 및 베이스(Bass) 채널 전용 딜레이/크로스오버 동기화 유틸리티입니다.
- **주요 기능:** 메인 스피커와의 위상차/시간차 밀리초 단위 정밀 보정(Delay Sync), 맞춤형 컷오프(Cut-off) 필터 지원.
- **기술 스택:** Tauri v2, Rust, SvelteKit, TailwindCSS.

### 3. Vesper Gate (`/gate`)
Cloudflare Pingora 및 Tokio 기반의 초경량 로컬 리버스 프록시, API 게이트웨이 & 안티 디도스 방패입니다.
- **주요 기능:** 포트 없는 도메인 라우팅, CORS 1초 자동 해결, 램 캐싱, 가짜 JSON Mock API, 윈도우 시스템 트레이 무창 상주 및 글래스모피즘 웹 대시보드.
- **기술 스택:** Rust, Tokio, Hyper, Pingora 아키텍처, Win32 API.

## 🚀 로컬 개발 및 빌드 (Development & Build)

### Vesper Gate 빌드
```bash
cd gate
cargo build --release
```

## 📜 오픈소스 고지 및 라이선스 (Credits & License)

- **AutoEq:** Copyright (c) 2018 Jaakko Pasanen (MIT License)
- **Spinorama:** Copyright (c) 2020 Pierre Aubert (MIT License)

### Proprietary License
**Copyright (c) 2026 Vesper (minseokk77). All Rights Reserved.**
본 저장소의 모든 소스코드와 지적재산권은 원작자에게 귀속되며, 허가 없는 무단 복제, 배포, 리버스 엔지니어링 및 상업적 이용을 엄격히 금지합니다.
