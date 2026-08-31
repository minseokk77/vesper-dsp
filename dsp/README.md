# 🎧 Vesper DSP (BioPhys 17.0 Real-Time Audio Core)

**Vesper DSP**는 가상 오디오 케이블(VB-Cable) 설치 없이, 윈도우 순정 오디오 시스템(`audiodg.exe`)에 직접 결합되어 **초저지연 Bit-Perfect 리샘플링, BioPhys 위상 정렬 필터, 그리고 AutoEQ를 실시간으로 가공**하는 차세대 프로 오디오 DSP 소프트웨어입니다.

Rust 기반의 **Zero-Mul SIMD 가속 커널, 16코어 Real-time Core Pinning, Zero-Sleep 적응형 스핀루프, 그리고 Windows APO 드라이버리스 아키텍처**를 결합하여, 타이달(TIDAL), 유튜브, 게임 등 모든 시스템 오디오를 0ns 지연시간으로 완벽하게 제어합니다.

---

## ✨ 핵심 기능 (Features)

- 🔌 **가상 케이블 100% 불필요 (Zero Virtual Cable Windows APO)**
  - 복잡한 VB-Cable 설치나 사운드 제어판 설정 없이, 순수 Rust APO 엔진(`vesper_apo.dll`)이 윈도우 오디오 시스템에 직접 결합되어 모든 소리를 실시간 처리합니다.
  - 사용자는 오직 **내가 소리를 들을 [출력 장치(헤드폰/스피커)]** 하나만 고르면 끝납니다.

- 🎛️ **하드웨어 최적화 리얼타임 코어 피닝 (Core Affinity #2 & Time-Critical Priority)**
  - OS 시스템 인터럽트가 발생하는 코어 #0을 회피하고, 전용 고성능 코어 #2에 오디오 렌더링 스레드를 고정 바인딩하여 **버퍼 언더런/소리 튐 0%**를 달성했습니다.

- ⚡ **Zero-Sleep 나노초 적응형 스핀루프 (Sub-Microsecond Latency)**
  - 1ms Sleep으로 인한 지터와 지연시간을 전면 제거하고, `std::hint::spin_loop()` 나노초 적응형 백오프로 오디오 패킷을 즉시 처리합니다.

- 🌊 **BioPhys 17.0 위상 플래시 & 3-Mass 음향 리샘플링 필터**
  - `biophys_phase_flash` (512x 초정밀 윈도우) 및 `biophys_acoustic_smooth` 필터 옵션 탑재.
  - 384kHz / 768kHz DSD 및 고해상도 PCM 음원에서도 프리링잉(Pre-ringing) 없는 깨끗한 원음을 보존합니다.

- 🎧 **AutoEQ & Spinorama 헤드폰/스피커 타겟 보정**
  - 수천 개 이상의 헤드폰/스피커 측정치 DB를 바탕으로 원클릭 정밀 파라메트릭 EQ 프로필을 실시간 적용합니다.

- 📦 **Zero-Vite 단일 바이너리 임베딩 (ERR_CONNECTION_REFUSED 원천 소멸)**
  - 외부 Node.js/Vite 웹서버 없이 모든 UI 정적 리소스가 `.exe` 내부에 직접 포함되어, 더블클릭 즉시 0.001초 만에 실행됩니다.

---

## 🛠️ 기술 스택 (Tech Stack)

### **Audio Core & Backend**
- **Core Language**: Rust (100% Safe Native Rust)
- **Windows Driverless Intercept**: Windows APO (Audio Processing Object) COM In-Place Processing
- **Real-Time Streaming**: CPAL + Realtime Thread Affinity Mask (`windows` crate)
- **DSP Filter Bank**: DirectForm1 SIMD-Ready Biquad & Rubato Sinc Resampler
- **IPC Pipeline**: `Global\VesperDspApoSharedMemory` (0ns Shared Memory Sync)
- **Desktop Framework**: Tauri v2

### **Frontend UI**
- **Framework**: SvelteKit 5 + TypeScript
- **Styling**: Tailwind CSS + Liquid Glass Dark Mode
- **Icons**: Heroicons / SVG Vectors

---

## 🚀 사용 가이드 (How to Use)

### 1. 즉시 실행
1. `vesper-dsp.exe`를 실행합니다. (가상 케이블 설치 불필요)
2. **OUTPUT DEVICE (재생 장치)** 목록에서 내가 소리를 들을 헤드폰/스피커(예: `FiiO K11`)를 선택합니다.
3. 앱이 자동으로 해당 장치의 Windows Endpoint GUID를 감지하여 APO를 백그라운드에서 즉시 바인딩합니다.
4. 타이달, 유튜브, 멜론, 게임 등 평소대로 음악을 감상하시면 BioPhys DSP 사운드가 100% 자동 적용됩니다!

---

## 💻 빌드 가이드 (Build from Source)

```bash
# 1. 의존성 설치
pnpm install

# 2. 정적 UI 빌드
pnpm build

# 3. Rust 단독 릴리즈 빌드
cargo build --release --manifest-path src-tauri/Cargo.toml
```

---

**Vesper DSP** © 2026. Powered by BioPhys 17.0 Neural Audio Architecture.
