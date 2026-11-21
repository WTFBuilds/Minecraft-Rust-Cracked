import { Globe, Star } from "lucide-react";
import { Reveal } from "@/components/reveal";
import { GITHUB_URL } from "@/components/site-nav";

export function CtaBand() {
  return (
    <section className="pb-24 pt-4 sm:pb-28">
      <div className="container-x">
        <Reveal>
          <div className="pixel-card hard-shadow relative overflow-hidden px-6 py-16 text-center sm:px-12">
            <div className="pointer-events-none absolute -left-24 -top-24 h-64 w-64 rounded-full bg-melon-600/10 blur-[80px]" />
            <div className="pointer-events-none absolute -bottom-24 -right-24 h-64 w-64 rounded-full bg-melon-700/10 blur-[80px]" />
            <div className="pointer-events-none absolute left-10 top-10 hidden h-3 w-3 animate-float bg-night-600 sm:block" />
            <div className="pointer-events-none absolute bottom-12 right-12 hidden h-3 w-3 animate-float border border-line-bright bg-night-800 [animation-delay:1.6s] sm:block" />

            <p className="eyebrow relative text-fog">
              06 // Join the harvest
            </p>
            <h2 className="relative mx-auto mt-4 max-w-2xl font-display text-3xl font-bold tracking-tight text-white sm:text-4xl">
              Witness the future of arcade infrastructure.
            </h2>
            <p className="relative mx-auto mt-4 max-w-xl text-sm leading-relaxed text-fog sm:text-base">
              MELON is in active development. Star the repository, watch the
              roadmap land crate by crate, and help build the arcade era.
            </p>
            <div className="relative mt-8 flex flex-wrap items-center justify-center gap-3">
              <a
                href={GITHUB_URL}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-2 border-2 border-white bg-white px-6 py-3 font-display text-sm font-bold uppercase tracking-wider text-night-950 shadow-[4px_4px_0_0_rgba(0,0,0,0.55)] transition-all hover:-translate-x-0.5 hover:-translate-y-0.5 hover:shadow-[6px_6px_0_0_rgba(0,0,0,0.55)]"
              >
                <Star className="h-4 w-4 fill-current" />
                Star on GitHub
              </a>
              <a
                href="https://melon.wtfbuilds.com"
                className="inline-flex items-center gap-2 border-2 border-line-bright bg-night-900 px-6 py-3 font-display text-sm font-bold uppercase tracking-wider text-mist transition-colors hover:border-white/40 hover:text-white"
              >
                <Globe className="h-4 w-4" />
                melon.wtfbuilds.com
              </a>
            </div>
          </div>
        </Reveal>
      </div>
    </section>
  );
}
