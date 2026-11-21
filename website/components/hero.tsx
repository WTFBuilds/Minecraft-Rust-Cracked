"use client";

import { useEffect, useState } from "react";
import { ChevronDown, Pause, Play, RotateCcw, Star } from "lucide-react";
import { GITHUB_URL } from "@/components/site-nav";

const SIM_TICK_MS = 200; // 5 ticks/s wall-clock = 20 TPS at 4 ticks/step
const BLOCKS = 20;

function useTickSimulator() {
  const [ticks, setTicks] = useState(0);
  const [entities, setEntities] = useState(184);
  const [running, setRunning] = useState(true);

  useEffect(() => {
    if (!running) return;
    const id = setInterval(() => {
      setTicks((t) => t + 4);
      setEntities((e) => {
        const next = e + Math.floor(Math.random() * 9) - 4;
        return Math.min(512, Math.max(96, next));
      });
    }, SIM_TICK_MS);
    return () => clearInterval(id);
  }, [running]);

  const reset = () => {
    setTicks(0);
    setEntities(184);
  };

  return { ticks, entities, running, setRunning, reset };
}

function TickSimulator() {
  const { ticks, entities, running, setRunning, reset } = useTickSimulator();
  const active = ticks % BLOCKS;

  return (
    <div className="pixel-card hard-shadow w-full max-w-3xl p-5 sm:p-6">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b-2 border-line pb-4">
        <div className="flex items-center gap-2.5">
          <span className="relative flex h-2.5 w-2.5">
            <span
              className={`absolute inline-flex h-full w-full ${running ? "animate-ping bg-melon-400" : ""} opacity-60`}
            />
            <span
              className={`relative inline-flex h-2.5 w-2.5 ${running ? "bg-melon-400" : "bg-flesh-500"}`}
            />
          </span>
          <p className="font-mono text-[11px] uppercase tracking-[0.2em] text-fog">
            Runtime concept visualization
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => setRunning((r) => !r)}
            className="inline-flex items-center gap-1.5 border-2 border-line-bright bg-night-700 px-3 py-1.5 font-mono text-[11px] uppercase tracking-wider text-mist transition-colors hover:border-melon-600 hover:text-melon-300"
          >
            {running ? <Pause className="h-3 w-3" /> : <Play className="h-3 w-3" />}
            {running ? "Pause" : "Resume"}
          </button>
          <button
            type="button"
            onClick={() => {
              setRunning(false);
              setTimeout(() => {
                reset();
                setRunning(true);
              }, 120);
            }}
            className="inline-flex items-center gap-1.5 border-2 border-line-bright bg-night-700 px-3 py-1.5 font-mono text-[11px] uppercase tracking-wider text-mist transition-colors hover:border-melon-600 hover:text-melon-300"
          >
            <RotateCcw className="h-3 w-3" />
            Reset
          </button>
        </div>
      </div>

      <div className="grid grid-cols-3 gap-3 py-5 sm:gap-4">
        <div className="border-2 border-line bg-night-800 px-3 py-3 text-center sm:py-4">
          <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-fog">
            Tick
          </p>
          <p className="mt-1 font-mono text-lg font-bold text-white tabular-nums sm:text-2xl">
            {ticks.toLocaleString("en-US")}
          </p>
        </div>
        <div className="border-2 border-line-bright bg-night-800 px-3 py-3 text-center sm:py-4">
          <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-fog">
            Tick rate
          </p>
          <p className="mt-1 font-mono text-lg font-bold text-melon-400 tabular-nums sm:text-2xl">
            20.0
          </p>
        </div>
        <div className="border-2 border-line bg-night-800 px-3 py-3 text-center sm:py-4">
          <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-fog">
            Entities
          </p>
          <p className="mt-1 font-mono text-lg font-bold text-white tabular-nums sm:text-2xl">
            {entities}
          </p>
        </div>
      </div>

      <div className="flex items-end gap-1 sm:gap-1.5">
        {Array.from({ length: BLOCKS }, (_, i) => (
          <div
            key={i}
            className={`h-6 flex-1 border-2 transition-colors duration-150 sm:h-7 ${
              i === active
                ? "border-melon-400 bg-melon-400"
                : (active - i + BLOCKS) % BLOCKS < 3
                  ? "border-line-bright bg-night-700"
                  : "border-line bg-night-800"
            }`}
          />
        ))}
      </div>
      <p className="mt-3 font-mono text-[10px] leading-relaxed text-fog/80">
        Stylized simulation of a 20 TPS game loop — design target, not a live
        benchmark. Public numbers ship with the benchmark suite.
      </p>
    </div>
  );
}

export function Hero() {
  return (
    <section id="top" className="relative overflow-hidden pb-20 pt-32 sm:pt-40">
      {/* Ambient glows */}
      <div className="pointer-events-none absolute -top-40 left-1/2 h-96 w-[42rem] -translate-x-1/2 rounded-full bg-melon-600/10 blur-[110px]" />
      <div className="pointer-events-none absolute -right-24 top-1/3 h-72 w-72 rounded-full bg-melon-700/10 blur-[90px]" />
      {/* Floating pixel blocks */}
      <div className="pointer-events-none absolute left-[8%] top-[22%] hidden h-4 w-4 animate-float bg-night-600 lg:block" />
      <div className="pointer-events-none absolute right-[12%] top-[18%] hidden h-3 w-3 animate-float bg-melon-600/50 [animation-delay:1.2s] lg:block" />
      <div className="pointer-events-none absolute left-[16%] top-[64%] hidden h-5 w-5 animate-float border-2 border-line-bright bg-night-800 [animation-delay:2.4s] lg:block" />
      <div className="pointer-events-none absolute right-[7%] top-[62%] hidden h-3.5 w-3.5 animate-float bg-flesh-500/40 [animation-delay:0.8s] lg:block" />

      <div className="container-x relative flex flex-col items-center text-center">
        <div className="inline-flex items-center gap-2.5 border-2 border-line-bright bg-night-900/80 px-4 py-1.5">
          <span className="h-2 w-2 animate-blink bg-melon-400" />
          <span className="font-mono text-[11px] uppercase tracking-[0.2em] text-mist">
            A WTFBUILDS project · In development
          </span>
        </div>

        <h1 className="mt-7 max-w-3xl font-display text-4xl font-bold leading-[1.08] tracking-tight text-white sm:text-6xl">
          Minecraft Arcade Infrastructure,{" "}
          <span className="text-melon-400">Reimagined.</span>
        </h1>

        <p className="mt-6 max-w-2xl text-base leading-relaxed text-fog sm:text-lg">
          A high-performance Minecraft Java Edition 1.8.9 arcade server engine,
          written in Rust. Purpose-built to power BedWars and competitive
          minigames with maximum efficiency and minimal hosting cost.
        </p>

        <div className="mt-8 flex flex-wrap items-center justify-center gap-3">
          <a
            href={GITHUB_URL}
            target="_blank"
            rel="noreferrer"
            className="inline-flex items-center gap-2 border-2 border-white bg-white px-6 py-3 font-display text-sm font-bold uppercase tracking-wider text-night-950 shadow-[4px_4px_0_0_rgba(0,0,0,0.55)] transition-all hover:-translate-x-0.5 hover:-translate-y-0.5 hover:shadow-[6px_6px_0_0_rgba(0,0,0,0.55)]"
          >
            <Star className="h-4 w-4 fill-current" />
            View on GitHub
          </a>
          <a
            href="#roadmap"
            className="inline-flex items-center gap-2 border-2 border-line-bright bg-night-900 px-6 py-3 font-display text-sm font-bold uppercase tracking-wider text-mist transition-colors hover:border-melon-600 hover:text-melon-300"
          >
            Explore the roadmap
            <ChevronDown className="h-4 w-4" />
          </a>
        </div>

        <div className="mt-12 flex w-full justify-center">
          <TickSimulator />
        </div>

        <div className="mt-8 flex flex-wrap items-center justify-center gap-2 font-mono text-[11px] uppercase tracking-[0.16em] text-fog">
          {["Protocol 47", "Minecraft 1.8.9", "Rust native", "Isolated match runtime"].map(
            (chip) => (
              <span
                key={chip}
                className="border border-line-bright bg-night-900/70 px-3 py-1.5"
              >
                {chip}
              </span>
            ),
          )}
        </div>
      </div>
    </section>
  );
}
