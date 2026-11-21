"use client";

import { useEffect, useState } from "react";
import { Menu, Star, X } from "lucide-react";
import { GithubIcon } from "@/components/github-icon";
import { MelonMark } from "@/components/melon-mark";

export const GITHUB_URL = "https://github.com/WTFBuilds/Minecraft-Rust-Cracked";

const LINKS = [
  { href: "#features", label: "Features" },
  { href: "#modes", label: "Game Modes" },
  { href: "#compare", label: "Compare" },
  { href: "#architecture", label: "Architecture" },
  { href: "#roadmap", label: "Roadmap" },
];

export function SiteNav() {
  const [scrolled, setScrolled] = useState(false);
  const [open, setOpen] = useState(false);

  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 12);
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <header
      className={`fixed inset-x-0 top-0 z-50 transition-colors duration-300 ${
        scrolled || open
          ? "border-b-2 border-line bg-night-950/90 backdrop-blur-md"
          : "border-b-2 border-transparent"
      }`}
    >
      <nav className="container-x flex h-16 items-center justify-between gap-4">
        <a href="#top" className="flex items-center gap-2.5">
          <MelonMark className="h-8 w-8" />
          <span className="font-display text-lg font-bold tracking-wide text-white">
            MELON
          </span>
          <span className="hidden rounded-sm border border-line-bright bg-night-800 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-widest text-fog sm:inline">
            Engine
          </span>
        </a>

        <div className="hidden items-center gap-1 lg:flex">
          {LINKS.map((link) => (
            <a
              key={link.href}
              href={link.href}
              className="rounded-sm px-3 py-2 text-sm font-medium text-fog transition-colors hover:text-white"
            >
              {link.label}
            </a>
          ))}
        </div>

        <div className="flex items-center gap-2">
          <a
            href={GITHUB_URL}
            target="_blank"
            rel="noreferrer"
            aria-label="GitHub repository"
            className="hidden h-10 w-10 items-center justify-center border-2 border-line bg-night-900 text-fog transition-colors hover:border-line-bright hover:text-white sm:flex"
          >
            <GithubIcon className="h-4.5 w-4.5" />
          </a>
          <a
            href={GITHUB_URL}
            target="_blank"
            rel="noreferrer"
            className="hidden items-center gap-2 border-2 border-line-bright bg-night-900 px-4 py-2 font-display text-xs font-bold uppercase tracking-wider text-white shadow-[3px_3px_0_0_rgba(0,0,0,0.5)] transition-all hover:-translate-y-px hover:border-white/40 hover:shadow-[4px_4px_0_0_rgba(0,0,0,0.5)] sm:inline-flex"
          >
            <Star className="h-3.5 w-3.5 fill-current" />
            Star
          </a>
          <button
            type="button"
            aria-label={open ? "Close menu" : "Open menu"}
            onClick={() => setOpen((v) => !v)}
            className="flex h-10 w-10 items-center justify-center border-2 border-line bg-night-900 text-mist lg:hidden"
          >
            {open ? <X className="h-4.5 w-4.5" /> : <Menu className="h-4.5 w-4.5" />}
          </button>
        </div>
      </nav>

      {open ? (
        <div className="border-t-2 border-line bg-night-950 lg:hidden">
          <div className="container-x flex flex-col py-3">
            {LINKS.map((link) => (
              <a
                key={link.href}
                href={link.href}
                onClick={() => setOpen(false)}
                className="py-3 text-sm font-medium text-fog transition-colors hover:text-white"
              >
                {link.label}
              </a>
            ))}
            <a
              href={GITHUB_URL}
              target="_blank"
              rel="noreferrer"
              className="mt-2 inline-flex items-center gap-2 border-2 border-white bg-white px-4 py-3 font-display text-xs font-bold uppercase tracking-wider text-night-950"
            >
              <GithubIcon className="h-4 w-4" />
              Star on GitHub
            </a>
          </div>
        </div>
      ) : null}
    </header>
  );
}
