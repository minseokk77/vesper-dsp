# 🎧 Vesper DSP (Private Source Repository)

**Vesper DSP**는 오디오 애호가와 전문가를 위해 설계된 **고성능 시스템 와이드 디지털 시그널 프로세서(DSP)** 및 오디오 엔진입니다. 
Tauri와 Rust 기반의 강력한 백엔드를 통해 Windows 운영체제 환경에서 무손실에 가까운 초저지연 오디오 필터링과 리샘플링을 제공합니다.

⚠️ **이 저장소는 Vesper DSP의 핵심 소스코드를 포함하는 비공개(Private) 저장소입니다.**
배포용 설치 파일 및 업데이터(Updater) 서버는 공개 저장소인 [minseokk77/vesper-dsp](https://github.com/minseokk77/vesper-dsp)에서 관리됩니다.

## 🛠️ 기술 스택 (Tech Stack)
- **Frontend UI:** SvelteKit, TypeScript, TailwindCSS (Vite)
- **Backend (Core):** Rust, Tauri v2
- **Audio Processing:** `cpal`, `biquad`, `rubato` (Real-time Resampling & EQ)

## ✨ 주요 기능 (Features)
- 🎛️ **강력한 오디오 라우팅 및 처리:** 가상 오디오 케이블(VAC)을 통한 자유로운 Input/Output 디바이스 매핑
- 🎚️ **AutoEQ 및 Spinorama 통합 지원:** 전 세계 수천 개의 헤드폰/이어폰 및 스피커 측정치 기반 주파수 보정 데이터 원클릭 통합 검색 및 자동 적용
- 🚀 **OS 친화적이고 가벼운 백그라운드 구동:** Windows 부팅 시 자동 시작 및 시스템 트레이 최소화 지원
- 🎨 **세련된 UI/UX:** 윈도우 11 환경에 최적화된 다크 모드와 글래스모피즘(Glassmorphism) 디자인

## 🚀 로컬 개발 및 빌드 (Development & Build)

### 사전 요구사항 (Prerequisites)
- Node.js (v18+)
- pnpm
- Rust (최신 안정화 버전)
- Tauri v2 개발 환경 (C++ Build Tools 등)

### 개발 서버 실행 (Run Dev)
프론트엔드와 백엔드 컴파일을 동시에 수행하며 실시간 핫리로드(Hot-reload)를 지원합니다.
```bash
cd dsp
pnpm install
pnpm tauri dev
```

### 릴리즈 빌드 (Build Release)
배포용 Windows 설치 파일(`.exe` / `.msi`)을 생성합니다.
빌드된 파일은 `dsp/src-tauri/target/release/bundle/nsis/` 경로에 저장됩니다.
```bash
cd dsp
pnpm tauri build
```
*(참고: 빌드 후 배포 시에는 `pnpm tauri signer sign` 명령을 통해 `.sig` 보안 서명을 생성한 뒤 공개 저장소의 `updater.json`에 갱신해야 합니다.)*

## 📜 오픈소스 고지 및 라이선스 (Credits & License)

- **AutoEq:** Copyright (c) 2018 Jaakko Pasanen (MIT License)
- **Spinorama:** Copyright (c) 2020 Pierre Aubert (MIT License)

### Proprietary License
**Copyright (c) 2026 Vesper (minseokk77). All Rights Reserved.**
본 저장소의 모든 소스코드와 지적재산권은 원작자에게 귀속되며, 허가 없는 무단 복제, 배포, 리버스 엔지니어링 및 상업적 이용을 엄격히 금지합니다.
