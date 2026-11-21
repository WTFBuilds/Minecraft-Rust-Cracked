import type { Metadata, Viewport } from "next";
import { Inter, JetBrains_Mono, Space_Grotesk } from "next/font/google";
import "./globals.css";

export const SITE_URL = "https://melon.wtfbuilds.com";

const display = Space_Grotesk({
  subsets: ["latin"],
  variable: "--font-space-grotesk",
});

const body = Inter({
  subsets: ["latin"],
  variable: "--font-inter",
});

const mono = JetBrains_Mono({
  subsets: ["latin"],
  variable: "--font-jetbrains",
});

export const metadata: Metadata = {
  metadataBase: new URL(SITE_URL),
  title: {
    default: "MELON — Minecraft Arcade Infrastructure, Reimagined",
    template: "%s — MELON",
  },
  description:
    "A high-performance Minecraft Java Edition 1.8.9 arcade server engine, written in Rust. Purpose-built for BedWars and competitive minigames. More players. More matches. Less infrastructure.",
  applicationName: "MELON",
  authors: [{ name: "WTFBUILDS", url: "https://wtfbuilds.com" }],
  creator: "WTFBUILDS",
  publisher: "WTFBUILDS",
  category: "technology",
  keywords: [
    "MELON",
    "Minecraft server engine",
    "Minecraft server Rust",
    "BedWars server",
    "SkyWars",
    "Duels",
    "Minecraft 1.8.9",
    "Protocol 47",
    "arcade server",
    "game engine",
    "high performance Minecraft",
    "WTFBUILDS",
  ],
  alternates: {
    canonical: "/",
  },
  openGraph: {
    title: "MELON — Minecraft Arcade Infrastructure, Reimagined",
    description:
      "A high-performance Minecraft Java Edition 1.8.9 arcade server engine, written in Rust. Purpose-built for BedWars and competitive minigames.",
    url: "/",
    siteName: "MELON",
    locale: "en_US",
    images: [
      {
        url: "/og-image.png",
        width: 1672,
        height: 941,
        alt: "MELON — Minecraft Arcade Infrastructure, Reimagined",
      },
    ],
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "MELON — Minecraft Arcade Infrastructure, Reimagined",
    description:
      "A high-performance Minecraft Java Edition 1.8.9 arcade server engine, written in Rust.",
    images: ["/og-image.png"],
  },
  robots: {
    index: true,
    follow: true,
    googleBot: {
      index: true,
      follow: true,
      "max-image-preview": "large",
      "max-snippet": -1,
      "max-video-preview": -1,
    },
  },
};

export const viewport: Viewport = {
  themeColor: "#060a06",
  colorScheme: "dark",
};

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html
      lang="en"
      className={`${display.variable} ${body.variable} ${mono.variable}`}
    >
      <body>{children}</body>
    </html>
  );
}
