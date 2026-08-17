# ⚡ Vesper Gate (`vgate`)

> **Vesper 생태계의 초고속 로컬 리버스 프록시, API 게이트웨이 & 보안 쉴드(WAF)**
> 
> Cloudflare Pingora 및 Tokio 기반의 압도적인 초경량·고성능 Rust 엔진 탑재

---

## 🌟 주요 기능

1. **마우스 클릭 & 토글 스위치 중심의 풀 UI**:
   * **🛡️ 침입 탐지 & 보안 쉴드(WAF) 토글**: 해킹 봇, `.env` 탈취, SQLi 시도 실시간 감지 & 차단
   * **📺 치지직/스트리밍 완충 부스터 토글**: 다음 영상 조각 3초 치를 미리 램에 확보하여 끊김 박멸
   * **CORS 자동 해결 토글**: 스위치 하나로 프론트/백엔드 통신 에러 1초 해결
   * **초고속 RAM 캐싱 토글**: 동일 요청 메모리 0.1ms 응답 모드
   * **부팅 시 자동 시작 토글**: 컴퓨터를 켤 때 백그라운드 자동 가동
   * **도메인 / 포트 연결 입력창**: `app.local` ➜ `5173` 입력 후 [연결 추가] 클릭
   * **Mock API 생성 입력창**: 경로와 가짜 JSON 입력 후 [Mock 등록] 클릭
   * **윈도우 hosts 복사 도우미**: 등록된 도메인 목록을 원클릭으로 클립보드에 복사
2. **무창 & 시스템 트레이(숨겨진 아이콘) 상주**:
   * 부팅 시 백그라운드 엔진이 0.001초 만에 먼저 뜨고, UI는 사용자가 원할 때 트레이 아이콘을 통해 오픈
3. **윈도우 제어판 정식 등록 & 원클릭 제거**:
   * 윈도우 설정의 **[설치된 앱] / [프로그램 추가/제거]**에 `Vesper Gate`로 정식 등록되어 [제거] 버튼으로 흔적 없이 삭제 가능

---

## 💻 사용 방법

1. [`vgate.exe`](file:///C:/Users/minse/Documents/antigravity/noble-babbage/vesper-gate/vgate.exe)를 실행합니다.
2. 윈도우 우측 하단 **[숨겨진 아이콘 (시스템 트레이)]**에 상주합니다.
3. 트레이 아이콘 우클릭 ➡️ **[🌐 웹 대시보드 열기]**를 누르면 글래스모피즘 관리 화면이 열립니다.

---

## 📜 라이선스 및 오픈소스 고지 (License & Credits)

* **Vesper Proprietary License (EULA)**: Copyright (c) 2026 minseokk77. All Rights Reserved.
  * 개인적/비상업적 용도로 완전 무료로 사용 가능합니다.
  * 무단 상업적 재판매, 역공학 및 바이너리 수정은 금지됩니다.
* **Third-Party Open Source Credits**:
  * [Cloudflare Pingora](https://github.com/cloudflare/pingora) (Apache-2.0 License)
  * [Tokio](https://tokio.rs) & [Hyper](https://hyper.rs) (MIT License)
