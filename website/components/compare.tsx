import { Target, Check } from "lucide-react";
import { Reveal } from "@/components/reveal";
import { SectionHead } from "@/components/section-head";

type Mark = "check" | "target";

const ROWS: {
  capability: string;
  traditional: string;
  melon: string;
  melonMark: Mark;
}[] = [
  {
    capability: "Design focus",
    traditional: "General-purpose platform, decades of compatibility surface",
    melon: "Arcade-only engine with a narrow protocol target",
    melonMark: "check",
  },
  {
    capability: "Runtime",
    traditional: "JVM instance per server",
    melon: "Native Rust runtime",
    melonMark: "check",
  },
  {
    capability: "Per-match footprint",
    traditional: "Heavy per-instance memory overhead",
    melon: "Minimal by design",
    melonMark: "target",
  },
  {
    capability: "Match isolation",
    traditional: "Shared worlds or a process per map",
    melon: "Isolated matches in one runtime",
    melonMark: "target",
  },
  {
    capability: "Scaling model",
    traditional: "Add more machines",
    melon: "More matches per machine",
    melonMark: "target",
  },
  {
    capability: "Benchmarks",
    traditional: "Rarely published",
    melon: "Public, reproducible harnesses",
    melonMark: "target",
  },
];

function MelonMarkIcon({ mark }: { mark: Mark }) {
  if (mark === "check") {
    return (
      <span className="flex h-5 w-5 shrink-0 items-center justify-center border border-line-bright bg-night-800">
        <Check className="h-3 w-3 text-white" />
      </span>
    );
  }
  return (
    <span className="flex h-5 w-5 shrink-0 items-center justify-center border border-dashed border-line-bright bg-night-800">
      <Target className="h-3 w-3 text-fog" />
    </span>
  );
}

export function Compare() {
  return (
    <section id="compare" className="py-24 sm:py-28">
      <div className="container-x">
        <SectionHead
          index="03"
          eyebrow="The Difference"
          title={
            <>
              Traditional stacks <span className="text-fog">vs.</span> the MELON
              approach.
            </>
          }
        />

        <Reveal className="mt-14">
          <div className="pixel-card hard-shadow overflow-x-auto">
            <table className="w-full min-w-[640px] border-collapse text-left">
              <thead>
                <tr className="border-b-2 border-line font-mono text-[11px] uppercase tracking-[0.18em]">
                  <th className="px-5 py-4 font-medium text-fog">Capability</th>
                  <th className="px-5 py-4 font-medium text-fog">
                    Traditional Java stacks
                  </th>
                  <th className="bg-white/[0.04] px-5 py-4 font-medium text-white">
                    MELON
                  </th>
                </tr>
              </thead>
              <tbody>
                {ROWS.map((row, i) => (
                  <tr
                    key={row.capability}
                    className={`border-b border-line/70 align-top ${
                      i % 2 === 1 ? "bg-night-800/40" : ""
                    }`}
                  >
                    <td className="px-5 py-4 text-sm font-semibold text-white">
                      {row.capability}
                    </td>
                    <td className="px-5 py-4 text-sm leading-relaxed text-fog">
                      {row.traditional}
                    </td>
                    <td className="bg-white/[0.04] px-5 py-4">
                      <div className="flex items-start gap-2.5 text-sm leading-relaxed text-mist">
                        <MelonMarkIcon mark={row.melonMark} />
                        <span>{row.melon}</span>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <p className="mt-3 flex items-center justify-center gap-4 font-mono text-[10px] uppercase tracking-[0.16em] text-fog/80">
            <span className="inline-flex items-center gap-1.5">
              <span className="h-2.5 w-2.5 border border-line-bright bg-night-800" />
              shipped direction
            </span>
            <span className="inline-flex items-center gap-1.5">
              <span className="h-2.5 w-2.5 border border-dashed border-line-bright bg-night-800" />
              design target — engine in active development
            </span>
          </p>
        </Reveal>
      </div>
    </section>
  );
}
