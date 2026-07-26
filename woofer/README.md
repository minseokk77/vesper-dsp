# 🎧 Vesper Woofer

**Vesper Woofer**는 PC에서 하나의 오디오 소스를 이어폰(또는 메인 헤드폰)과 서브 우퍼(스피커) 등 **2개의 출력 기기로 지연 없이 동시에 복제(라우팅)**해주는 고성능 오디오 동기화 유틸리티입니다.

Rust 기반의 초저지연 오디오 엔진과 Liquid Glass(글래스모피즘) 디자인이 적용된 Svelte UI를 결합하여, 무거운 DAW 소프트웨어 없이도 강력한 크로스오버 네트워크와 위상 정렬 기능을 아주 가볍게 사용할 수 있습니다.

---

## ✨ 핵심 기능 (Features)

- 🚀 **초저지연 오디오 복제 (Zero-Latency Routing)**
  - Rust `cpal` 및 `asio-sys` 기반의 오디오 스트리밍을 통해 시스템 오디오를 메인 기기와 우퍼로 동시에 쏴줍니다.
  - **ASIO & WASAPI 모드 완벽 지원**: Bit-Perfect 전송과 고해상도 샘플레이트(최대 384kHz 등) 호환성을 제공합니다.

- 🎛️ **강력한 크로스오버 (Subwoofer Crossover)**
  - 우퍼 스피커로 전달되는 소리 중 불필요한 고음을 잘라내는 **Low Pass Filter (LPF)** 내장.
  - 40Hz ~ 200Hz 주파수 컷오프(Cut-off) 지원 및 **12/24 dB/Octave 슬로프(Slope)** 설정 가능.

- ⏱️ **위상 정렬 및 딜레이 보정 (Phase Delay)**
  - 이어폰과 스피커의 물리적 거리에 따른 소리 전달 속도 차이를 맞추기 위해 **0~1000ms 범위의 미세 딜레이 보정**을 지원합니다.

- 🎚️ **전문가용 DSP 설정 (Advanced DSP)**
  - 헤드룸 확보(Headroom dB) 및 오디오 클리핑(Clipping) 인디케이터 지원.
  - 고품질 리샘플링 알고리즘 (Linear / Sinc Interpolation) 선택.
  - DSD 필터 및 게인(Gain) 조절 기능.

- 💾 **원클릭 사운드 프리셋 (Presets)**
  - **Movie, Music, Gaming** 3가지 프로필에 입출력 기기 선택부터 크로스오버, 딜레이, 고급 DSP 설정까지 모조리 저장하여 원클릭으로 전환할 수 있습니다.

- ⌨️ **글로벌 단축키 (Global Hotkey)**
  - 윈도우 환경 어디서든 **`Ctrl + Alt + W`** 키를 눌러 오디오 싱크 엔진을 즉시 켜고 끌 수 있습니다.

- 🎨 **모던 Liquid Glass 디자인**
  - Svelte와 Tailwind CSS로 구현된 투명하고 아름다운 다크 모드 기반의 글래스모피즘 UI/UX를 제공합니다.

- 🔄 **무중단 자동 업데이트 (Auto Update)**
  - Tauri 백엔드를 통해 앱 실행 시 새 버전(GitHub Release)이 감지되면 인앱에서 원클릭으로 백그라운드 업데이트가 진행됩니다.

---

## 🛠 기술 스택 (Tech Stack)

### **Frontend**
- **Framework**: SvelteKit (SSG) + TypeScript + Vite
- **Styling**: Tailwind CSS + Custom Liquid Glass UI (CSS)
- **Icons**: Lucide Icons / Heroicons

### **Backend (Desktop Core)**
- **Framework**: Tauri v2
- **Language**: Rust
- **Audio Engine**: `cpal`, `asio-sys` (CPAL 커스텀 드라이버 연동)
- **Plugins**: 
  - `tauri-plugin-updater` (자동 업데이트)
  - `tauri-plugin-global-shortcut` (단축키 처리)
  - `tauri-plugin-autostart` (부팅 시 시작)
  - `window-vibrancy` (Windows 11 Acrylic/Mica 투명도 효과)

---

## 🚀 사용 가이드 (How to Use)

### 1. 가상 오디오 케이블 설치 준비
Vesper Woofer는 입력된 소리를 잡아채서 2곳으로 뿌려줍니다. 따라서 PC의 기본 출력 장치를 가상 케이블로 설정해야 합니다.
- 추천 가상 케이블: [VB-Audio Virtual Cable](https://vb-audio.com/Cable/) 또는 [Hi-Fi Cable](https://vb-audio.com/Cable/) 등.

### 2. 세팅 방법
1. 윈도우 기본 소리 출력 장치를 **`Hi-Fi Cable Input`**으로 설정합니다.
2. **Vesper Woofer** 앱을 실행합니다.
3. **AUDIO SOURCE** 란에서 방금 윈도우 기본 장치로 잡은 `Hi-Fi Cable Output`을 선택합니다.
4. **PRIMARY EARPHONES** 에 사용 중인 이어폰/헤드폰(또는 메인 DAC)을 선택합니다.
5. **SUB WOOFER** 에 베이스를 울려줄 서브 스피커를 선택합니다.
6. 맨 하단의 **ENGAGE SYNC** (또는 `Ctrl+Alt+W`)를 눌러 오디오 복제를 시작합니다!

### 3. 미세 조정
- **Low Pass Filter**: 스피커에서 고음이 같이 나와 거슬린다면 Hz(주파수)를 낮추고 Slope를 24 dB/Octave로 설정하세요.
- **Phase Delay**: 이어폰의 소리보다 스피커의 둥둥거림이 늦게 들린다면 스피커 위치에 맞게 딜레이(ms)를 조금씩 높여보세요.

---

## 💻 개발자 빌드 (Build from Source)

이 프로젝트를 로컬에서 빌드하려면 Node.js (pnpm) 및 Rust 툴체인이 필요합니다.

```bash
# 의존성 설치
pnpm install

# 개발 서버 실행 (Tauri 앱 구동)
pnpm tauri dev

# 프로덕션 빌드 (Windows .exe 인스톨러 생성)
pnpm tauri build
```

*(참고: 릴리즈 패키징을 위한 서명 키를 `.env` 파일에 셋업해야 `tauri build`의 자동 업데이트 서명(.sig)이 정상 생성됩니다.)*

---

**Vesper Woofer** © 2026. Made with ❤️ using Tauri & Svelte.
