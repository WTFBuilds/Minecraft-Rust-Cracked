# MELON — Landing Website

The official landing site for **MELON**, the Minecraft arcade server engine.
Built with [Next.js](https://nextjs.org) 16, React 19, and Tailwind CSS v4.

## Development

```bash
cd website
npm install
npm run dev
```

Open [http://localhost:3000](http://localhost:3000) in your browser.

## Production

```bash
npm run build
npm run start
```

## Structure

```text
website/
├── app/
│   ├── layout.tsx        # Fonts, SEO metadata, Open Graph
│   ├── page.tsx          # Section composition
│   ├── globals.css       # Theme tokens, pixel-card utilities, motion
│   └── icon.svg          # Pixel melon favicon
├── components/           # One component per landing section
│   ├── site-nav.tsx      # Sticky nav + mobile menu
│   ├── hero.tsx          # Headline + interactive tick simulator
│   ├── marquee.tsx       # Scrolling keyword strip
│   ├── features.tsx      # "Why MELON" feature grid
│   ├── modes.tsx         # Game modes (BedWars spotlight)
│   ├── compare.tsx       # MELON vs traditional stacks table
│   ├── architecture.tsx  # System flow + planned workspace tree
│   ├── roadmap.tsx       # Four-phase roadmap timeline
│   ├── cta-band.tsx      # Closing call to action
│   └── footer.tsx        # Link columns + disclaimer
└── public/               # Brand artwork (og-image, BedWars scene)
```

## Notes

- Fully static output — every route prerenders at build time.
- The hero tick simulator is a stylized concept visualization of the 20 TPS
  design target, not a live benchmark.
- Brand palette and type scale live in `app/globals.css` under `@theme`.
