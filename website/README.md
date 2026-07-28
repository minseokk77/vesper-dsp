# Vesper Web

Vesper DSP와 Vesper Woofer를 소개하고 배포하는 공식 웹사이트입니다.

## 제품

- **Vesper DSP**: Windows 전체 오디오를 헤드폰, 이어폰, 스피커에 맞게 보정
- **Vesper Woofer**: 2.1채널 시스템의 서브우퍼 딜레이와 크로스오버 정렬

## 기술 구성

- Next.js 16
- React 19
- TypeScript
- vinext / Cloudflare Workers

## 로컬 실행

Node.js 22.13 이상이 필요합니다.

```bash
npm ci
npm run dev
```

## 검증

```bash
npm test
npm run lint
```

프로덕션 웹사이트: [Vesper Audio Studio](https://vesper-audio-studio.minseok-7881.chatgpt.site)
