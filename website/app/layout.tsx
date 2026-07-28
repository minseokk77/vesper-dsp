import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  metadataBase: new URL("https://vesper.minseok.online"),
  title: "Vesper | Windows 오디오를 더 정교하게",
  description:
    "시스템 전역 DSP와 서브우퍼 딜레이 동기화를 하나의 정교한 Windows 오디오 경험으로 만나보세요.",
  icons: {
    icon: "/favicon.svg",
    shortcut: "/favicon.svg",
  },
  openGraph: {
    title: "Vesper | Your sound, refined.",
    description:
      "헤드폰·스피커 보정부터 서브우퍼 동기화까지. Windows 오디오를 더 정교하게.",
    type: "website",
    locale: "ko_KR",
    siteName: "Vesper",
    images: [
      {
        url: "/og.png",
        width: 1734,
        height: 907,
        alt: "Vesper 오디오 프로세싱 인터페이스",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: "Vesper | Your sound, refined.",
    description:
      "헤드폰·스피커 보정부터 서브우퍼 동기화까지. Windows 오디오를 더 정교하게.",
    images: ["/og.png"],
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="ko">
      <body
        className={`${geistSans.variable} ${geistMono.variable} antialiased`}
      >
        {children}
      </body>
    </html>
  );
}
