export const DEFAULT_TILE_SERVER =
  'https://tile.openstreetmap.org/{z}/{x}/{y}.png'

export const TILE_ATTRIBUTION =
  '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'

// Old defaults can remain in browser storage or saved tile-server entries.
// Preserve custom providers and CARTO URLs with explicit credentials/options.
export function resolveTileServer(url?: string) {
  if (
    !url ||
    /^https:\/\/(?:\{s\}|[abcd])\.basemaps\.cartocdn\.com\/(?:rastertiles\/voyager_labels_under|dark_all)\/\{z\}\/\{x\}\/\{y\}(?:\{r\})?\.png$/.test(
      url,
    )
  ) {
    return DEFAULT_TILE_SERVER
  }
  return url
}
