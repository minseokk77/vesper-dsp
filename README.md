# 🎧 Vesper DSP

**Vesper DSP**는 오디오 애호가와 전문가를 위해 설계된 **고성능 시스템 와이드 디지털 시그널 프로세서(DSP)** 및 오디오 엔진입니다. 
Tauri와 Rust 기반의 강력한 백엔드를 통해 Windows 운영체제 환경에서 무손실에 가까운 초저지연 오디오 필터링과 리샘플링을 제공합니다.

## ✨ 주요 기능 (Features)

- 🎛️ **강력한 오디오 라우팅 및 처리**
  - 가상 오디오 케이블(VAC)을 통한 자유로운 Input/Output 디바이스 매핑
  - `44.1kHz`부터 `384kHz` 이상까지 지원하는 전문가급 리샘플링 전략 (Linear, Sinc 등)
- 🎚️ **AutoEQ 통합 지원**
  - 전 세계 수천 개의 헤드폰/이어폰 측정치 기반 주파수 보정 데이터(AutoEQ) 원클릭 검색 및 자동 적용
- 🚀 **OS 친화적이고 가벼운 백그라운드 구동**
  - Windows 부팅 시 자동 시작(Auto-start) 지원
  - 이전 DSP 렌더링 상태를 영구 기억하여(Persistence) 앱 실행 시 즉각적인 오디오 복구
- 🎨 **세련된 UI/UX**
  - 윈도우 11 환경에 최적화된 고급스러운 다크 모드와 글래스모피즘(Glassmorphism) 디자인
  - 실시간 설정 핫 적용(Debouncing) 기능으로 끊김 없는 사운드 튜닝 경험 제공

## 📥 다운로드 및 설치 (Download & Install)

가장 최신의 Vesper DSP 설치 프로그램은 화면 우측의 **[Releases](https://github.com/minseokk7/vesper-dsp/releases)** 탭에서 다운로드하실 수 있습니다.

1. Releases 탭에서 `VesperDSP_x.x.x_x64-setup.exe` 파일을 다운로드하여 실행합니다.
2. 설치 완료 후 앱을 실행하면 백그라운드 엔진이 자동으로 켜지며 고음질 오디오 필터링 환경이 구축됩니다.

*(참고: 이 저장소는 Vesper DSP 앱의 설치 파일 배포 및 자동 업데이트(Tauri Updater) 서버로 동작하는 공개 공간입니다. 실제 핵심 오디오 프로세싱 소스코드는 보안 상 비공개 저장소에서 분리 관리되고 있습니다.)*

## 📜 오픈소스 고지 (Open Source Credits)

이 소프트웨어는 전 세계 헤드폰 및 이어폰의 주파수 응답 데이터를 수집하고 이퀄라이제이션(EQ) 프로파일을 제공하는 훌륭한 오픈소스 프로젝트인 **[AutoEq](https://github.com/jaakkopasanen/AutoEq)**의 데이터를 연동하여 사용하고 있습니다.

- **AutoEq** is licensed under the MIT License.
- Copyright (c) 2018 Jaakko Pasanen
- 원본 프로젝트 저장소: https://github.com/jaakkopasanen/AutoEq
