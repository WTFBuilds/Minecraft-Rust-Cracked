const ITEMS = [
  "BedWars",
  "SkyWars",
  "Duels",
  "Protocol 47",
  "20 TPS target",
  "Rust native",
  "Multi-match runtime",
  "Low-latency PvP",
  "Zero JVM",
  "WTFBUILDS",
];

export function Marquee() {
  return (
    <div className="overflow-hidden border-y-2 border-line bg-night-900 py-3.5">
      <div className="flex w-max animate-marquee items-center gap-8">
        {[0, 1].map((copy) => (
          <div
            key={copy}
            aria-hidden={copy === 1}
            className="flex items-center gap-8"
          >
            {ITEMS.map((item) => (
              <span key={item} className="flex items-center gap-8">
                <span className="font-mono text-xs font-semibold uppercase tracking-[0.24em] text-fog">
                  {item}
                </span>
                <span className="h-1.5 w-1.5 rotate-45 bg-line-bright" />
              </span>
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}
