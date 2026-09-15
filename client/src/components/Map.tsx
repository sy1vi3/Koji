import * as React from 'react'
import { usePersist } from '@hooks/usePersist'
import { MapContainer, TileLayer } from 'react-leaflet'
import { ATTRIBUTION } from '@assets/constants'
import { resolveTileServer, TILE_ATTRIBUTION } from '@services/tiles'

interface Props {
  children?: React.ReactNode
  forcedLocation?: [number, number]
  forcedZoom?: number
  style?: React.CSSProperties
  zoomControl?: boolean
  renderOwnTileLayer?: boolean
}

const Map = React.forwardRef<L.Map, Props>(
  (
    {
      children,
      forcedLocation,
      forcedZoom,
      style,
      zoomControl,
      renderOwnTileLayer,
    },
    ref,
  ) => {
    const { location, zoom } = usePersist.getState()
    const tileServer = usePersist((s) => resolveTileServer(s.tileServer))

    return (
      <MapContainer
        key="map"
        ref={ref}
        center={forcedLocation ?? location}
        zoom={forcedZoom ?? zoom}
        zoomControl={zoomControl}
        style={style}
        maxBounds={[
          [-85, -180],
          [85, 180],
        ]}
      >
        {!renderOwnTileLayer && (
          <TileLayer
            key={tileServer}
            attribution={`${ATTRIBUTION} | ${TILE_ATTRIBUTION}`}
            url={tileServer}
          />
        )}
        {children}
      </MapContainer>
    )
  },
)

Map.displayName = 'Map'

export default Map
