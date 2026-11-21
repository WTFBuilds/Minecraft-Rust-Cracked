import { Square } from "lucide-react";
import { Reveal } from "@/components/reveal";
import { SectionHead } from "@/components/section-head";

const PHASES = [
  {
    phase: "Phase 1",
    status: "In design",
    title: "Foundation",
    items: [
      "Project foundation & core architecture",
      "Minecraft 1.8.9 protocol implementation",
      "Client handshake and login",
    ],
  },
  {
    phase: "Phase 2",
    status: "Planned",
    title: "Engine Core",
    items: [
      "Player movement & synchronization",
      "World and chunk management",
      "Multi-match runtime",
    ],
  },
  {
    phase: "Phase 3",
    status: "Planned",
    title: "Game Layer",
    items: [
      "BedWars engine",
      "Combat, inventory & item systems",
      "Matchmaking and lobby flow",
    ],
  },
  {
    phase: "Phase 4",
    status: "Planned",
    title: "Scale & Launch",
    items: [
      "Performance benchmarking suite",
      "Production deployment",
      "SkyWars & Duels support",
      "Arcade orchestration infrastructure",
    ],
  },
];

export function Roadmap() {
  return (
    <section id="roadmap" className="py-24 sm:py-28">
      <div className="container-x">
        <SectionHead
          index="05"
          eyebrow="Roadmap"
          title="From foundation to arcade network."
          sub="Four phases, each shipping something runnable. Follow progress directly in the repository."
        />

        <div className="relative mt-14">
          <div className="absolute bottom-8 left-[19px] top-2 hidden w-0.5 bg-line sm:block" />
          <div className="space-y-4">
            {PHASES.map((phase, i) => (
              <Reveal key={phase.phase} delay={i * 80}>
                <div className="relative flex flex-col gap-4 sm:flex-row sm:items-start">
                  <div
                    className={`relative z-10 flex h-10 w-10 shrink-0 items-center justify-center border-2 font-mono text-xs font-bold ${
                      i === 0
                        ? "border-white bg-white text-night-950"
                        : "border-line-bright bg-night-900 text-fog"
                    }`}
                  >
                    {String(i + 1).padStart(2, "0")}
                  </div>
                  <div
                    className={`pixel-card w-full p-6 ${
                      i === 0
                        ? "border-line-bright shadow-[4px_4px_0_0_rgba(0,0,0,0.5)]"
                        : ""
                    }`}
                  >
                    <div className="flex flex-wrap items-center gap-3">
                      <h3 className="font-display text-lg font-bold text-white">
                        {phase.title}
                      </h3>
                      <span
                        className={`px-2.5 py-0.5 font-mono text-[10px] uppercase tracking-widest ${
                          i === 0
                            ? "border border-line-bright bg-night-800 text-white"
                            : "border border-line-bright bg-night-800 text-fog"
                        }`}
                      >
                        {phase.status}
                      </span>
                      <span className="font-mono text-[10px] uppercase tracking-widest text-fog/70">
                        {phase.phase}
                      </span>
                    </div>
                    <ul className="mt-4 grid gap-2.5 sm:grid-cols-2">
                      {phase.items.map((item) => (
                        <li
                          key={item}
                          className="flex items-center gap-2.5 text-sm text-fog"
                        >
                          <Square className="h-3 w-3 shrink-0 text-fog" />
                          {item}
                        </li>
                      ))}
                    </ul>
                  </div>
                </div>
              </Reveal>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
