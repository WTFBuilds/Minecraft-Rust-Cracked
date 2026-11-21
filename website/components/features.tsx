import { Cpu, Gamepad2, Gauge, Layers, TrendingUp, Zap } from "lucide-react";
import { Reveal } from "@/components/reveal";
import { SectionHead } from "@/components/section-head";

const FEATURES = [
  {
    icon: Zap,
    title: "Performance",
    desc: "Efficient game simulation and packet processing on the hot path — every tick counts.",
  },
  {
    icon: Cpu,
    title: "Resource Efficiency",
    desc: "Minimal CPU and memory overhead per player and per match, by design.",
  },
  {
    icon: Layers,
    title: "Massive Concurrency",
    desc: "Many isolated matches running in parallel — without interference between them.",
  },
  {
    icon: Gauge,
    title: "Low Latency",
    desc: "Sub-tick responsiveness for movement, combat, and knockback where it matters most.",
  },
  {
    icon: TrendingUp,
    title: "Scalability",
    desc: "Maximum concurrent players and matches per machine — grow without growing bills.",
  },
  {
    icon: Gamepad2,
    title: "Built for Arcade",
    desc: "Purpose-built for the workloads of BedWars, SkyWars, Duels, and beyond.",
  },
];

export function Features() {
  return (
    <section id="features" className="py-24 sm:py-28">
      <div className="container-x">
        <SectionHead
          index="01"
          eyebrow="Why MELON"
          title="Engineered for the arcade workload."
          sub="Traditional server architectures carry unnecessary overhead when hosting large numbers of independent matches. MELON strips it away."
        />

        <div className="mt-14 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {FEATURES.map((feature, i) => (
            <Reveal key={feature.title} delay={(i % 3) * 90}>
              <div className="pixel-card group h-full p-6 transition-all duration-200 hover:-translate-x-0.5 hover:-translate-y-0.5 hover:border-line-bright hover:shadow-[5px_5px_0_0_rgba(0,0,0,0.5)]">
                <div className="flex h-11 w-11 items-center justify-center border-2 border-line-bright bg-night-700 text-mist transition-colors group-hover:border-white/30">
                  <feature.icon className="h-5 w-5" />
                </div>
                <h3 className="mt-5 font-display text-lg font-bold text-white">
                  {feature.title}
                </h3>
                <p className="mt-2 text-sm leading-relaxed text-fog">
                  {feature.desc}
                </p>
              </div>
            </Reveal>
          ))}
        </div>
      </div>
    </section>
  );
}
