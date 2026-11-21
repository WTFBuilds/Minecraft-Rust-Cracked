import type { MetadataRoute } from "next";

export default function manifest(): MetadataRoute.Manifest {
  return {
    name: "MELON — Minecraft Arcade Infrastructure, Reimagined",
    short_name: "MELON",
    description:
      "A high-performance Minecraft Java Edition 1.8.9 arcade server engine, written in Rust.",
    start_url: "/",
    display: "standalone",
    background_color: "#060a06",
    theme_color: "#060a06",
    icons: [
      {
        src: "/icon.svg",
        sizes: "any",
        type: "image/svg+xml",
        purpose: "any",
      },
    ],
  };
}
