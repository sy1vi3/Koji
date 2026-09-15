import * as React from 'react'
import Map from '@components/Map'
import type { KojiTileServer } from '@assets/types'
import { TileLayer } from 'react-leaflet'
import { resolveTileServer, TILE_ATTRIBUTION } from '@services/tiles'

export default function TileServerMap({
  formData,
}: {
  formData: KojiTileServer
}) {
  try {
    return (
      <Map
        key={formData.url}
        renderOwnTileLayer
        style={{ width: '100%', height: '50vh' }}
      >
        <TileLayer
          url={resolveTileServer(formData.url)}
          attribution={TILE_ATTRIBUTION}
        />
      </Map>
    )
  } catch (e) {
    return <div>URL is invalid, could not load the preview</div>
  }
}
