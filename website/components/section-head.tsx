import type { ReactNode } from "react";
import { Reveal } from "@/components/reveal";

export function SectionHead({
  index,
  eyebrow,
  title,
  sub,
}: {
  index: string;
  eyebrow: string;
  title: ReactNode;
  sub?: string;
}) {
  return (
    <Reveal className="mx-auto max-w-2xl text-center">
      <p className="eyebrow text-fog">
        <span className="text-fog/60">{index}</span> {"//"} {eyebrow}
      </p>
      <h2 className="mt-4 font-display text-3xl font-bold tracking-tight text-white sm:text-4xl">
        {title}
      </h2>
      {sub ? <p className="mt-4 text-base leading-relaxed text-fog">{sub}</p> : null}
    </Reveal>
  );
}
