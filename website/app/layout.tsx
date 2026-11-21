import type { Metadata } from "next";
import { Inter, JetBrains_Mono, Space_Grotesk } from "next/font/google";
import "./globals.css";

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
  metadataBase: new URL("https://melon.wtfbuilds.com"),
  title: "MELON — Minecraft Arcade Infrastructure, Reimagined",
  description:
    "A high-performance Minecraft Java Edition 1.8.9 arcade server engine, written in Rust. More players. More matches. Less infrastructure.",
  keywords: [
    "MELON",
    "Minecraft server",
    "BedWars",
    "Rust",
    "arcade",
    "game engine",
    "WTFBUILDS",
  ],
  openGraph: {
    title: "MELON — Minecraft Arcade Infrastructure, Reimagined",
    description:
      "A high-performance Minecraft Java Edition 1.8.9 arcade server engine, written in Rust.",
    url: "/",
    siteName: "MELON",
    images: [{ url: "/og-image.png", width: 1672, height: 941 }],
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "MELON — Minecraft Arcade Infrastructure, Reimagined",
    description:
      "A high-performance Minecraft Java Edition 1.8.9 arcade server engine, written in Rust.",
    images: ["/og-image.png"],
  },
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
