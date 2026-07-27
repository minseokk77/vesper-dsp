export const en = {
  nav: {
    dsp: 'DSP',
    woofer: 'Woofer',
    github: 'GitHub'
  },
  hero: {
    badge: 'Available for Windows',
    title1: 'The Ultimate',
    title2: 'Audio Ecosystem.',
    subtitle: 'Experience studio-grade digital signal processing and flawless subwoofer delay synchronization, perfectly integrated for your desktop.',
    btnDsp: 'Explore DSP',
    btnWoofer: 'Explore Woofer'
  },
  home: {
    dspTitle: 'Vesper DSP',
    dspDesc: 'System-wide digital signal processor and virtual audio cable. Integrates seamlessly with AutoEq and Spinorama to provide pixel-perfect acoustic correction.',
    dspFeatures: ['Virtual Audio Routing', 'AutoEQ & Spinorama Profiles', 'Real-time Resampling'],
    wooferTitle: 'Vesper Woofer',
    wooferDesc: 'The ultimate subwoofer and bass channel delay synchronization utility. Fix phase cancellation and achieve perfect low-end harmony in any room acoustic environment.',
    wooferFeatures: ['Millisecond Precision Delay Sync', 'Custom Crossover & Cut-off Filters', 'Seamless DSP Integration']
  },
  dspDetail: {
    title: 'System-Wide Acoustic Correction',
    intro: 'Vesper DSP is not just a simple equalizer. It captures the entire Windows system audio output via a Virtual Audio Cable and reconstructs the sound in real-time within an ultra-low latency environment.',
    box1Title: 'AutoEq & Spinorama',
    box1Desc: 'Apply one-click AutoEq profiles based on over 9,000 headphone/earphone frequency measurement databases, allowing you to experience flat reference sound that overcomes hardware limitations.',
    box2Title: 'Real-time Processing',
    box2Desc: 'Utilizing Rust\'s cpal and rubato libraries, it provides ultra-fast audio routing and high-quality asynchronous resampling bypassing the Windows kernel stack. Absolutely zero lip-sync issues.',
    downloadBtn: 'Download DSP for Windows'
  },
  wooferDetail: {
    title: 'Perfect Bass Phase Alignment',
    intro: 'Vesper Woofer perfectly solves the chronic phase cancellation problems that occur in 2.1+ channel environments mixing main speakers and subwoofers entirely through software.',
    box1Title: 'Delay Synchronization',
    box1Desc: 'Correct the time alignment (Time Alignment) of sound reaching the listener\'s ear between the main speaker and subwoofer down to the millisecond (ms) to maximize low-end impact.',
    box2Title: 'Crossover Filtering',
    box2Desc: 'Through internal DSP Biquad filters, cleanly cut out the bandwidth the subwoofer will handle based on a specific frequency band (e.g., 80Hz) to suppress booming and create the tightest sub-bass.',
    downloadBtn: 'Download Woofer for Windows'
  },
  footer: {
    copyright: '© 2026 minseokk77',
    repo: 'Private Monorepo',
    dspRel: 'DSP Releases',
    wooRel: 'Woofer Releases',
    license: 'License (EULA)'
  },
  license: {
    title: 'Software License Agreement (EULA)',
    subtitle: 'End User License Agreement',
    terms: [
      {
        title: '1. Ownership and Copyright',
        content: 'All rights, title, and intellectual property rights in and to Vesper DSP and Vesper Woofer (the "Software") are owned by the developer (minseokk77). The Software is licensed, not sold. (All Rights Reserved)'
      },
      {
        title: '2. Permitted Use',
        content: 'This Software is provided free of charge for personal, non-commercial use only. Under no circumstances may the Software be resold or redistributed for commercial purposes.'
      },
      {
        title: '3. Restrictions',
        content: 'You may not reverse engineer, decompile, disassemble, or attempt to extract the source code of the Software. You also may not modify or create derivative works based on the Software in whole or in part.'
      },
      {
        title: '4. Disclaimer of Warranty',
        content: 'The Software is provided "AS IS", without warranty of any kind, express or implied, including but not limited to the warranties of merchantability or fitness for a particular purpose. The entire risk arising out of use of the Software remains with you.'
      }
    ]
  }
};
