import Image from "next/image";
import { Check, Lock, Radar, Swords } from "lucide-react";
import { Reveal } from "@/components/reveal";
import { SectionHead } from "@/components/section-head";

const BEDWARS_POINTS = [
  "Constant block place & break churn",
  "Projectile, fireball, and entity physics",
  "Latency-critical PvP combat and knockback",
  "Full arena resets between matches",
];

export function Modes() {
  return (
    <section id="modes" className="border-y-2 border-line bg-night-900/40 py-24 sm:py-28">
      <div className="container-x">
        <SectionHead
          index="02"
          eyebrow="Game Modes"
          title="BedWars first. Arcade infrastructure next."
          sub="BedWars stresses every part of the engine, making it the perfect proving ground. SkyWars and Duels follow on the same shared foundation."
        />

        <Reveal className="mt-14">
          <div className="pixel-card hard-shadow grid overflow-hidden lg:grid-cols-2">
            <div className="flex flex-col justify-center p-7 sm:p-10">
              <span className="inline-flex w-fit items-center gap-2 border-2 border-line-bright bg-night-800 px-3 py-1 font-mono text-[10px] uppercase tracking-[0.2em] text-white">
                Primary target
              </span>
              <h3 className="mt-4 font-display text-2xl font-bold text-white sm:text-3xl">
                BedWars
              </h3>
              <p className="mt-3 text-sm leading-relaxed text-fog sm:text-base">
                The most infrastructure-hungry mode in competitive Minecraft —
                and the first optimization target for the MELON runtime. Every
                system below is exercised at full intensity, all match long.
              </p>
              <ul className="mt-6 space-y-3">
                {BEDWARS_POINTS.map((point) => (
                  <li key={point} className="flex items-start gap-2.5 text-sm text-mist">
                    <span className="mt-0.5 flex h-4.5 w-4.5 shrink-0 items-center justify-center border border-line-bright bg-night-800">
                      <Check className="h-3 w-3 text-white" />
                    </span>
                    {point}
                  </li>
                ))}
              </ul>
            </div>
            <div className="relative min-h-64 border-t-2 border-line lg:border-l-2 lg:border-t-0">
              <Image
                src="/bedwars-scene.png"
                alt="Stylized BedWars arena artwork — red and blue teams bridging across floating islands"
                fill
                sizes="(min-width: 1024px) 50vw, 100vw"
                className="object-cover"
              />
              <div className="absolute inset-0 bg-gradient-to-t from-night-900/70 via-transparent to-transparent lg:bg-gradient-to-r" />
            </div>
          </div>
        </Reveal>

        <div className="mt-4 grid gap-4 sm:grid-cols-2">
          {[
            {
              icon: Radar,
              name: "SkyWars",
              desc: "Loot-driven island combat on the shared MELON runtime.",
            },
            {
              icon: Swords,
              name: "Duels",
              desc: "1v1 and 2v2 arena combat with tournament-grade latency.",
            },
          ].map((mode, i) => (
            <Reveal key={mode.name} delay={i * 100}>
              <div className="pixel-card flex items-start gap-4 border-dashed p-6 opacity-80 transition-opacity hover:opacity-100">
                <div className="flex h-11 w-11 shrink-0 items-center justify-center border-2 border-line bg-night-800 text-fog">
                  <mode.icon className="h-5 w-5" />
                </div>
                <div>
                  <div className="flex items-center gap-3">
                    <h3 className="font-display text-lg font-bold text-white">
                      {mode.name}
                    </h3>
                    <span className="inline-flex items-center gap-1.5 border border-line-bright bg-night-800 px-2 py-0.5 font-mono text-[10px] uppercase tracking-widest text-fog">
                      <Lock className="h-3 w-3" />
                      Planned
                    </span>
                  </div>
                  <p className="mt-1.5 text-sm leading-relaxed text-fog">
                    {mode.desc}
                  </p>
                </div>
              </div>
            </Reveal>
          ))}
        </div>
      </div>
    </section>
  );
}
