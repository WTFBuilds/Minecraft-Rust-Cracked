"use cache";

import type { MetadataRoute } from "next";

const SITE_URL = "https://melon.wtfbuilds.com";

export default async function sitemap(): Promise<MetadataRoute.Sitemap> {
  return [
    {
      url: SITE_URL,
      lastModified: new Date(),
      changeFrequency: "weekly",
      priority: 1,
    },
  ];
}
