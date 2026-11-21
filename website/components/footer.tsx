import { Globe } from "lucide-react";
import { GithubIcon } from "@/components/github-icon";
import { MelonMark } from "@/components/melon-mark";
import { GITHUB_URL } from "@/components/site-nav";

const COLUMNS = [
  {
    heading: "Project",
    links: [
      { label: "Features", href: "#features" },
      { label: "Game Modes", href: "#modes" },
      { label: "Compare", href: "#compare" },
      { label: "Architecture", href: "#architecture" },
      { label: "Roadmap", href: "#roadmap" },
    ],
  },
  {
    heading: "Game Modes",
    links: [
      { label: "BedWars", href: "#modes" },
      { label: "SkyWars (planned)", href: "#modes" },
      { label: "Duels (planned)", href: "#modes" },
    ],
  },
  {
    heading: "WTFBUILDS",
    links: [
      { label: "wtfbuilds.com", href: "https://wtfbuilds.com" },
      { label: "MELON site", href: "https://melon.wtfbuilds.com" },
      { label: "GitHub organization", href: "https://github.com/WTFBuilds" },
    ],
  },
  {
    heading: "Engine",
    links: [
      { label: "Rust language", href: "https://www.rust-lang.org" },
      { label: "Protocol 47 reference", href: "https://wiki.vg/Protocol" },
      { label: "MIT License", href: "https://github.com/WTFBuilds/Minecraft-Rust-Cracked/blob/main/LICENSE" },
    ],
  },
];

export function Footer() {
  return (
    <footer className="border-t-2 border-line bg-night-900/60">
      <div className="container-x grid gap-10 py-14 lg:grid-cols-[1.4fr_2fr]">
        <div>
          <a href="#top" className="flex items-center gap-2.5">
            <MelonMark className="h-8 w-8" />
            <span className="font-display text-lg font-bold tracking-wide text-white">
              MELON
            </span>
          </a>
          <p className="mt-4 max-w-xs text-sm leading-relaxed text-fog">
            Minecraft Arcade Infrastructure, Reimagined. More players. More
            matches. Less infrastructure.
          </p>
          <div className="mt-5 flex items-center gap-2">
            <a
              href={GITHUB_URL}
              target="_blank"
              rel="noreferrer"
              aria-label="GitHub repository"
              className="flex h-9 w-9 items-center justify-center border-2 border-line bg-night-900 text-fog transition-colors hover:border-white/40 hover:text-white"
            >
              <GithubIcon className="h-4 w-4" />
            </a>
            <a
              href="https://melon.wtfbuilds.com"
              aria-label="MELON website"
              className="flex h-9 w-9 items-center justify-center border-2 border-line bg-night-900 text-fog transition-colors hover:border-white/40 hover:text-white"
            >
              <Globe className="h-4 w-4" />
            </a>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-8 sm:grid-cols-4">
          {COLUMNS.map((column) => (
            <div key={column.heading}>
              <h3 className="font-mono text-[11px] font-semibold uppercase tracking-[0.2em] text-white">
                {column.heading}
              </h3>
              <ul className="mt-4 space-y-2.5">
                {column.links.map((link) => (
                  <li key={link.label}>
                    <a
                      href={link.href}
                      {...(link.href.startsWith("http")
                        ? { target: "_blank", rel: "noreferrer" }
                        : {})}
                      className="text-sm text-fog transition-colors hover:text-white"
                    >
                      {link.label}
                    </a>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>
      </div>

      <div className="border-t-2 border-line">
        <div className="container-x flex flex-col gap-3 py-6 sm:flex-row sm:items-center sm:justify-between">
          <p className="font-mono text-[11px] text-fog/80">
            © 2026 WTFBUILDS — Witness The Future Builds. MIT Licensed.
          </p>
          <p className="max-w-md font-mono text-[10px] leading-relaxed text-fog/60">
            MELON is not an official Minecraft product. Not approved by, or
            associated with, Mojang Studios or Microsoft.
          </p>
        </div>
      </div>
    </footer>
  );
}
