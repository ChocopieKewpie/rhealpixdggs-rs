import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

const base = "/rhealpixdggs-rs";

export default defineConfig({
  site: "https://chocopiekewpie.github.io",
  base,
  publicDir: "./docs",
  trailingSlash: "always",
  integrations: [
    starlight({
      title: "rHEALPix DGGS",
      description:
        "Fast aperture-9 rHEALPix indexing, coverage, and topology for Rust and Python.",
      logo: {
        src: "./src/assets/logo-mark.svg",
        alt: "rHEALPix DGGS",
      },
      favicon: "/images/logo-mark.svg",
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/ChocopieKewpie/rhealpixdggs-rs",
        },
      ],
      editLink: {
        baseUrl: "https://github.com/ChocopieKewpie/rhealpixdggs-rs/edit/main/",
      },
      lastUpdated: true,
      credits: true,
      customCss: ["./src/styles/custom.css"],
      head: [
        {
          tag: "meta",
          attrs: { name: "theme-color", content: "#0a5f68" },
        },
      ],
      sidebar: [
        {
          label: "Start here",
          items: [
            { slug: "index", label: "Overview" },
            { slug: "getting-started/installation", label: "Installation" },
            { slug: "getting-started/python", label: "Python quickstart" },
            { slug: "getting-started/rust", label: "Rust quickstart" },
          ],
        },
        {
          label: "Understand the grid",
          items: [
            { slug: "concepts/grid", label: "How rHEALPix works" },
            { slug: "concepts/coordinates", label: "Coordinates & boundaries" },
            { slug: "concepts/cell-ids", label: "Cell identifiers" },
            { slug: "concepts/coverage", label: "Coverage semantics" },
          ],
        },
        {
          label: "Cookbook",
          items: [
            { slug: "recipes", label: "Recipes overview" },
            { slug: "recipes/cas-crash-density", label: "CAS crash density" },
            { slug: "guides/polygon-to-gpkg", label: "Polygon to GeoPackage" },
          ],
        },
        {
          label: "API reference",
          collapsed: false,
          items: [
            { slug: "api", label: "API overview" },
            { slug: "api/indexing", label: "Indexing" },
            { slug: "api/geometry", label: "Cell geometry" },
            { slug: "api/hierarchy", label: "Hierarchy & identifiers" },
            { slug: "api/topology", label: "Topology & traversal" },
            { slug: "api/coverage", label: "Region coverage" },
            { slug: "api/compaction", label: "Compaction" },
            { slug: "api/numpy", label: "NumPy" },
            { slug: "api/geo", label: "GeoPandas & GeoPackage" },
            { slug: "api/compat", label: "Python compatibility facade" },
            { slug: "api/rust", label: "Rust crate" },
          ],
        },
        {
          label: "Engineering",
          collapsed: true,
          items: [
            { slug: "engineering/architecture", label: "Architecture" },
            { slug: "engineering/api-status", label: "Implementation status" },
            { slug: "engineering/numerical-accuracy", label: "Numerical accuracy" },
            {
              slug: "engineering/upstream-compatibility",
              label: "Upstream compatibility",
            },
            { slug: "engineering/upstream-v0-7-audit", label: "Upstream v0.7 audit" },
            { slug: "engineering/development", label: "Development" },
            { slug: "engineering/natural-earth-data", label: "Figure data" },
          ],
        },
      ],
    }),
  ],
});
