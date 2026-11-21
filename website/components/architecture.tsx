import { ArrowRight, GitBranch } from "lucide-react";
import { Reveal } from "@/components/reveal";
import { SectionHead } from "@/components/section-head";

const FLOW = [
  { title: "Minecraft 1.8.9 Clients", caption: "Java Edition" },
  { title: "Network Layer", caption: "TCP transport" },
  { title: "Protocol 47 Codec", caption: "Packet pipeline" },
  { title: "Game Runtime", caption: "Deterministic tick loop" },
  { title: "Match Manager", caption: "Allocation & lifecycle" },
];

const FOUNDATION = [
  { title: "World & Entities", caption: "Blocks, chunks, simulation" },
  { title: "Combat Core", caption: "Hit detection & knockback" },
  { title: "Resource Scheduler", caption: "Density per machine" },
];

const TREE = `melon/
├── crates/
│   ├── protocol/      # Protocol 47 codec
│   ├── networking/    # TCP & packet pipeline
│   ├── runtime/       # Game loops & scheduling
│   ├── world/         # Blocks & chunk state
│   ├── entities/      # Player & entity sim
│   ├── combat/        # PvP & knockback
│   ├── matchmaking/   # Match lifecycle
│   └── game-modes/    # BedWars, SkyWars, Duels
├── benchmarks/        # Reproducible harnesses
├── docs/              # Design documents
└── tests/             # Protocol & integration`;

export function Architecture() {
  return (
    <section
      id="architecture"
      className="border-y-2 border-line bg-night-900/40 py-24 sm:py-28"
    >
      <div className="container-x">
        <SectionHead
          index="04"
          eyebrow="System Design"
          title="One runtime. Every match."
          sub="A modular, multi-crate engine: packets flow through the protocol pipeline into a deterministic game loop that hosts isolated matches in parallel."
        />

        <Reveal className="mt-14">
          <div className="flex flex-wrap items-stretch justify-center gap-2">
            {FLOW.map((node, i) => (
              <div key={node.title} className="flex items-center">
                <div className="pixel-card w-40 p-3.5 text-center transition-colors hover:border-line-bright sm:w-44">
                  <p className="font-mono text-[10px] uppercase tracking-[0.16em] text-fog">
                    {String(i + 1).padStart(2, "0")}
                  </p>
                  <p className="mt-1.5 font-display text-sm font-bold leading-tight text-white">
                    {node.title}
                  </p>
                  <p className="mt-1 font-mono text-[10px] text-fog">
                    {node.caption}
                  </p>
                </div>
                {i < FLOW.length - 1 ? (
                  <ArrowRight className="mx-1 h-4 w-4 shrink-0 text-fog" />
                ) : null}
              </div>
            ))}
          </div>

          <div className="mx-auto mt-5 w-fit border-x-2 border-dashed border-line-bright px-8 py-2">
            <p className="font-mono text-[10px] uppercase tracking-[0.2em] text-fog">
              shared foundation
            </p>
          </div>

          <div className="mt-2 flex flex-wrap items-stretch justify-center gap-2">
            {FOUNDATION.map((node) => (
              <div
                key={node.title}
                className="pixel-card w-48 p-3.5 text-center transition-colors hover:border-line-bright"
              >
                <p className="font-display text-sm font-bold text-white">
                  {node.title}
                </p>
                <p className="mt-1 font-mono text-[10px] text-fog">
                  {node.caption}
                </p>
              </div>
            ))}
          </div>
        </Reveal>

        <div className="mx-auto mt-10 grid max-w-4xl gap-4 lg:grid-cols-[1fr_auto]">
          <Reveal>
            <div className="pixel-card hard-shadow h-full p-6">
              <div className="flex items-center gap-2.5">
                <GitBranch className="h-4 w-4 text-melon-400" />
                <h3 className="font-display text-base font-bold text-white">
                  Modular workspace
                </h3>
              </div>
              <p className="mt-2 text-sm leading-relaxed text-fog">
                Each subsystem is a standalone crate — protocol, transport,
                simulation, and game logic stay decoupled and independently
                testable.
              </p>
              <div className="mt-5 grid grid-cols-2 gap-2 font-mono text-[11px] text-fog sm:grid-cols-4">
                {["No unsafe defaults", "Async I/O core", "Tick schedulers", "Crate isolation"].map(
                  (chip) => (
                    <span
                      key={chip}
                      className="border border-line-bright bg-night-800 px-2.5 py-1.5 text-center"
                    >
                      {chip}
                    </span>
                  ),
                )}
              </div>
            </div>
          </Reveal>
          <Reveal delay={120}>
            <div className="pixel-card hard-shadow overflow-hidden">
              <div className="flex items-center gap-1.5 border-b-2 border-line bg-night-800 px-4 py-2.5">
                <span className="h-2.5 w-2.5 rounded-full bg-flesh-500/70" />
                <span className="h-2.5 w-2.5 rounded-full bg-melon-600/70" />
                <span className="h-2.5 w-2.5 rounded-full bg-melon-400/70" />
                <span className="ml-2 font-mono text-[10px] text-fog">
                  planned workspace
                </span>
              </div>
              <pre className="overflow-x-auto p-4 font-mono text-[11px] leading-relaxed text-fog">
                {TREE}
              </pre>
            </div>
          </Reveal>
        </div>
      </div>
    </section>
  );
}
