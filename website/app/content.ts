export type Language = "ko" | "en";

export const content = {
  ko: {
    skip: "본문으로 이동",
    navLabel: "주요 탐색",
    homeLabel: "Vesper 홈",
    nav: {
      products: "제품",
      features: "기능",
      faq: "FAQ",
    },
    switchLabel: "Switch to English",
    switchText: "EN",
    hero: {
      availability: "Windows 오디오 · 두 가지 정밀 도구",
      titleLineOne: "PC SOUND.",
      titleLineTwo: "TUNED TO YOUR SYSTEM.",
      descriptionLineOne: "헤드폰·스피커 보정과 서브우퍼 타이밍을 더 정확하게.",
      descriptionLineTwo:
        "필요한 제품을 선택하면 실제 인터페이스와 핵심 기능을 바로 확인할 수 있습니다.",
      primaryAction: "제품 비교하기",
      secondaryAction: "작동 방식 보기",
      note: "무료 · Windows 10/11 · x64",
    },
    preview: {
      label: "Vesper Audio 앱 화면 미리보기",
      title: "Vesper Audio",
      library: "라이브러리",
      overview: "전체 보기",
      equalizer: "이퀄라이저",
      devices: "장치",
      settings: "설정",
      eyebrow: "통합 오디오",
      heading: "하나의 사운드 시스템",
      processing: "처리 중",
      output: "출력",
      profile: "프로파일",
      active: "AutoEQ · 활성",
      preamp: "프리앰프",
    },
    statement: {
      label: "TWO APPS. ONE ECOSYSTEM.",
      lineOne: "필요한 도구는 서로 달라도,",
      lineTwo: "완성되는 소리는 하나입니다.",
      ariaLabel: "Vesper 소개",
    },
    products: {
      dsp: {
        category: "시스템 전역 음향 보정",
        description:
          "AutoEQ와 Spinorama 프로파일을 활용해 Windows의 모든 소리를 헤드폰과 스피커에 맞게 재구성합니다.",
        features: [
          "9,000+ 측정 프로파일",
          "최대 768 kHz · 장치별 16/24/32-bit",
          "실시간 비동기 리샘플링",
        ],
        action: "DSP 다운로드",
      },
      woofer: {
        category: "서브우퍼 타임 얼라인먼트",
        description:
          "스피커와 서브우퍼의 도달 시간과 크로스오버를 맞춰 흐릿했던 저역을 더 단단하고 정확하게 만듭니다.",
        features: [
          "밀리초 단위 딜레이 보정",
          "정밀 Biquad 크로스오버",
          "독립 실행형 Rust 앱",
        ],
        action: "Woofer 다운로드",
      },
    },
    experience: {
      label: "ENGINEERED TO DISAPPEAR",
      titleLineOne: "보이지 않게 정교하고,",
      titleLineTwo: "들을수록 분명합니다.",
      description:
        "설정은 직관적으로, 처리는 정밀하게. Vesper의 모든 기능은 음악에 더 가까이 다가가기 위해 존재합니다.",
      features: [
        {
          icon: "≈",
          title: "시스템 전역 보정",
          description:
            "브라우저부터 게임까지, Windows에서 재생되는 모든 소리를 한 번에 보정합니다.",
        },
        {
          icon: "⌁",
          title: "초저지연 프로세싱",
          description:
            "Rust 기반 오디오 엔진이 음악과 영상의 흐름을 깨뜨리지 않고 실시간으로 처리합니다.",
        },
        {
          icon: "◉",
          title: "정밀한 저역 정렬",
          description:
            "스피커와 서브우퍼의 시간차를 밀리초 단위로 맞춰 단단하고 자연스러운 저음을 만듭니다.",
        },
      ],
    },
    setup: {
      label: "READY IN MINUTES",
      title: "필요한 앱을 고르고, 바로 들어보세요.",
      description:
        "헤드폰·이어폰·스피커 보정에는 DSP, 2.1채널 서브우퍼 정렬에는 Woofer를 선택하세요. DSP는 VB-Audio Hi-Fi Cable 설치가 필요합니다.",
      action: "제품 선택하기",
      steps: [
        {
          title: "용도 선택",
          description: "헤드폰·스피커 보정 또는 서브우퍼 정렬을 선택하세요.",
        },
        {
          title: "앱 다운로드",
          description: "각 제품의 GitHub Releases에서 최신 버전을 받으세요.",
        },
        {
          title: "오디오 설정",
          description: "장치와 프로파일을 지정하면 Vesper가 처리합니다.",
        },
      ],
    },
    faq: {
      label: "GOOD TO KNOW",
      title: "자주 묻는 질문",
      items: [
        {
          question: "무료로 사용할 수 있나요?",
          answer:
            "네. Vesper DSP와 Vesper Woofer는 모두 무료로 다운로드할 수 있습니다.",
        },
        {
          question: "어떤 Windows 버전이 필요한가요?",
          answer:
            "Windows 10 또는 Windows 11의 64비트 환경을 지원합니다. DSP는 VB-Audio Hi-Fi Cable 설치가 추가로 필요합니다.",
        },
        {
          question: "DSP와 Woofer를 함께 설치해야 하나요?",
          answer:
            "아닙니다. 헤드폰·이어폰·스피커 보정에는 DSP만, 2.1채널 시스템의 서브우퍼 정렬에는 Woofer만 사용할 수 있습니다.",
        },
      ],
    },
    final: {
      label: "YOUR SOUND, REFINED.",
      title: "당신에게 필요한 Vesper를 선택하세요.",
      action: "DSP와 Woofer 비교하기",
    },
    footer: "© 2026 minseokk77. 더 나은 청취를 위해 만들었습니다.",
  },
  en: {
    skip: "Skip to content",
    navLabel: "Primary navigation",
    homeLabel: "Vesper home",
    nav: {
      products: "Products",
      features: "Features",
      faq: "FAQ",
    },
    switchLabel: "한국어로 전환",
    switchText: "KO",
    hero: {
      availability: "Windows audio · two precision tools",
      titleLineOne: "PC SOUND.",
      titleLineTwo: "TUNED TO YOUR SYSTEM.",
      descriptionLineOne: "Refine headphone, speaker, and subwoofer response.",
      descriptionLineTwo:
        "Choose a product to see its real interface and core capabilities.",
      primaryAction: "Compare products",
      secondaryAction: "See how it works",
      note: "Free · Windows 10/11 · x64",
    },
    preview: {
      label: "Vesper Audio application preview",
      title: "Vesper Audio",
      library: "Library",
      overview: "Overview",
      equalizer: "Equalizer",
      devices: "Devices",
      settings: "Settings",
      eyebrow: "UNIFIED AUDIO",
      heading: "One sound system",
      processing: "Processing",
      output: "Output",
      profile: "Profile",
      active: "AutoEQ · Active",
      preamp: "Preamp",
    },
    statement: {
      label: "TWO APPS. ONE ECOSYSTEM.",
      lineOne: "Different tools for different needs.",
      lineTwo: "One sound when they are done.",
      ariaLabel: "About Vesper",
    },
    products: {
      dsp: {
        category: "System-wide acoustic correction",
        description:
          "Use AutoEQ and Spinorama profiles to reshape every sound in Windows for your headphones and speakers.",
        features: [
          "9,000+ measurement profiles",
          "Up to 768 kHz · device-native bit depth",
          "Real-time asynchronous resampling",
        ],
        action: "Download DSP",
      },
      woofer: {
        category: "Subwoofer time alignment",
        description:
          "Align speaker and subwoofer arrival times and crossover behavior for tighter, more accurate bass.",
        features: [
          "Millisecond delay correction",
          "Precision Biquad crossover",
          "Standalone Rust application",
        ],
        action: "Download Woofer",
      },
    },
    experience: {
      label: "ENGINEERED TO DISAPPEAR",
      titleLineOne: "Invisible in operation.",
      titleLineTwo: "Obvious when you listen.",
      description:
        "Intuitive to configure and precise in operation. Every Vesper feature exists to bring you closer to the music.",
      features: [
        {
          icon: "≈",
          title: "System-wide correction",
          description:
            "Correct every sound played through Windows, from browsers to games, in one place.",
        },
        {
          icon: "⌁",
          title: "Ultra-low latency",
          description:
            "A Rust audio engine processes sound in real time without disrupting music or video.",
        },
        {
          icon: "◉",
          title: "Precise bass alignment",
          description:
            "Match speaker and subwoofer arrival times to create tight, natural low frequencies.",
        },
      ],
    },
    setup: {
      label: "READY IN MINUTES",
      title: "Choose what you need. Start listening.",
      description:
        "Pick DSP for headphone or speaker correction, or Woofer for subwoofer alignment in a 2.1 system. DSP requires VB-Audio Hi-Fi Cable.",
      action: "Choose a product",
      steps: [
        {
          title: "Choose your use case",
          description: "Select headphone or speaker correction, or subwoofer alignment.",
        },
        {
          title: "Download the app",
          description: "Get the latest version from each product's GitHub Releases.",
        },
        {
          title: "Configure audio",
          description: "Choose your device and profile, then let Vesper process it.",
        },
      ],
    },
    faq: {
      label: "GOOD TO KNOW",
      title: "Frequently asked questions",
      items: [
        {
          question: "Can I use Vesper for free?",
          answer:
            "Yes. Both Vesper DSP and Vesper Woofer are available as free downloads.",
        },
        {
          question: "Which versions of Windows are supported?",
          answer:
            "Vesper supports 64-bit Windows 10 and Windows 11. DSP also requires VB-Audio Hi-Fi Cable.",
        },
        {
          question: "Do I need to install both apps?",
          answer:
            "No. Use DSP for headphone, earphone, or speaker correction, and Woofer for subwoofer alignment in a 2.1 system.",
        },
      ],
    },
    final: {
      label: "YOUR SOUND, REFINED.",
      title: "Choose the Vesper app that fits your sound.",
      action: "Compare DSP and Woofer",
    },
    footer: "© 2026 minseokk77. Built for better listening.",
  },
} as const;
