use futures::{Stream, TryStreamExt};
use geo::{Contains, MultiPolygon, Point, Polygon};
use geojson::{FeatureCollection, Value};
use sea_orm::DbErr;
use serde_json::json;

use super::*;

use crate::{
    api::text::TextHelpers,
    db::{AreaRef, Spawnpoint, sea_orm_active_enums::Type},
};

pub trait HasLatLon {
    fn lat(&self) -> f64;
    fn lon(&self) -> f64;
}

impl HasLatLon for Spawnpoint {
    fn lat(&self) -> f64 {
        self.lat
    }
    fn lon(&self) -> f64 {
        self.lon
    }
}

impl HasLatLon for api::point_struct::PointStruct {
    fn lat(&self) -> f64 {
        self.lat
    }
    fn lon(&self) -> f64 {
        self.lon
    }
}

pub struct AreaPolygons {
    polys: Vec<Polygon<f64>>,
    multi_polys: Vec<MultiPolygon<f64>>,
}

impl AreaPolygons {
    pub fn from_collection(area: &FeatureCollection) -> Self {
        let mut polys = Vec::new();
        let mut multi_polys = Vec::new();

        for feature in &area.features {
            if let Some(geometry) = &feature.geometry {
                match &geometry.value {
                    Value::Polygon(_) => match Polygon::try_from(geometry) {
                        Ok(poly) => polys.push(poly),
                        Err(e) => log::warn!("Failed to convert Polygon: {}", e),
                    },
                    Value::MultiPolygon(_) => match MultiPolygon::try_from(geometry) {
                        Ok(mp) => multi_polys.push(mp),
                        Err(e) => log::warn!("Failed to convert MultiPolygon: {}", e),
                    },
                    _ => {}
                }
            }
        }

        Self { polys, multi_polys }
    }

    pub fn contains(&self, lat: f64, lon: f64) -> bool {
        let point = Point::new(lon, lat);
        self.polys.iter().any(|poly| poly.contains(&point))
            || self.multi_polys.iter().any(|mp| mp.contains(&point))
    }
}

/// Apply the cap after polygon filtering, without buffering all bounding-box rows.
pub async fn collect_in_area<T: HasLatLon>(
    mut rows: impl Stream<Item = Result<T, DbErr>> + Unpin,
    area: &FeatureCollection,
    point_limit: u64,
) -> Result<Vec<T>, DbErr> {
    let polygons = AreaPolygons::from_collection(area);
    let mut points = Vec::new();
    while let Some(point) = rows.try_next().await? {
        if polygons.contains(point.lat(), point.lon()) {
            points.push(point);
            if point_limit != 0 && points.len() as u64 >= point_limit {
                break;
            }
        }
    }
    Ok(points)
}

pub fn fort(items: api::single_struct::SingleStruct, prefix: &str) -> Vec<db::GenericData> {
    items
        .into_iter()
        .enumerate()
        .map(|(i, item)| db::GenericData::new(format!("{}{}", prefix, i), item.lat, item.lon))
        .collect()
}

pub fn fort_filtered(
    items: api::single_struct::SingleStruct,
    area: &FeatureCollection,
    prefix: &str,
) -> Vec<db::GenericData> {
    let polygons = AreaPolygons::from_collection(area);
    items
        .into_iter()
        .filter(|item| polygons.contains(item.lat(), item.lon()))
        .enumerate()
        .map(|(i, item)| db::GenericData::new(format!("{}{}", prefix, i), item.lat, item.lon))
        .collect()
}

pub fn count_in_area<T: HasLatLon>(items: &[T], area: &FeatureCollection) -> i32 {
    let polygons = AreaPolygons::from_collection(area);
    items
        .iter()
        .filter(|item| polygons.contains(item.lat(), item.lon()))
        .count() as i32
}

pub fn spawnpoint(items: Vec<db::Spawnpoint>) -> Vec<db::GenericData> {
    items
        .into_iter()
        .enumerate()
        .map(|(i, item)| {
            db::GenericData::new(
                format!(
                    "{}{}",
                    if item.despawn_sec.is_some() { "v" } else { "u" },
                    i
                ),
                item.lat,
                item.lon,
            )
        })
        .collect()
}

pub fn spawnpoint_filtered(
    items: Vec<Spawnpoint>,
    area: &FeatureCollection,
) -> Vec<db::GenericData> {
    let polygons = AreaPolygons::from_collection(area);
    items
        .into_iter()
        .filter(|item| polygons.contains(item.lat(), item.lon()))
        .enumerate()
        .map(|(i, item)| {
            db::GenericData::new(
                format!(
                    "{}{}",
                    if item.despawn_sec.is_some() { "v" } else { "u" },
                    i
                ),
                item.lat,
                item.lon,
            )
        })
        .collect()
}

pub fn instance(instance: db::instance::Model) -> Feature {
    instance
        .data
        .parse_scanner_instance(Some(instance.name), Some(instance.r#type))
}

pub fn area(areas: Vec<db::area::Model>) -> Vec<Feature> {
    let mut normalized = Vec::<Feature>::new();

    let mut to_feature = |fence: Option<String>, name: &String, mode: Type| {
        if let Some(fence) = fence {
            if !fence.is_empty() {
                normalized.push(fence.parse_scanner_instance(Some(name.to_string()), Some(mode)));
            }
        }
    };
    for area in areas.into_iter() {
        to_feature(area.geofence, &area.name, Type::AutoQuest);
        to_feature(area.fort_mode_route, &area.name, Type::CircleRaid);
        to_feature(area.quest_mode_route, &area.name, Type::CircleQuest);
        to_feature(area.pokemon_mode_route, &area.name, Type::CirclePokemon);
    }
    normalized
}

pub fn area_ref(areas: Vec<AreaRef>) -> Vec<sea_orm::JsonValue> {
    let mut normalized = Vec::<sea_orm::JsonValue>::new();

    for area in areas.into_iter() {
        if area.has_geofence {
            normalized.push(json!({
                "id": area.id,
                "name": area.name,
                "mode": "auto_quest",
                "geo_type": "MultiPolygon",
            }));
        }
        if area.has_fort {
            normalized.push(json!({
                "id": area.id,
                "name": area.name,
                "mode": "circle_raid",
                "geo_type": "MultiPoint",
            }));
        }
        if area.has_pokemon {
            normalized.push(json!({
                "id": area.id,
                "name": area.name,
                "mode": "circle_pokemon",
                "geo_type": "MultiPoint",
            }));
        }
        if area.has_quest {
            normalized.push(json!({
                "id": area.id,
                "name": area.name,
                "mode": "circle_quest",
                "geo_type": "MultiPoint",
            }));
        }
    }
    normalized
}

#[cfg(test)]
mod point_limit_tests {
    use super::*;
    use crate::api::args::{Args, BoundsArg, DEFAULT_POINT_LIMIT, resolve_point_limit};
    use futures::{executor::block_on, stream};

    fn area() -> FeatureCollection {
        serde_json::from_value(json!({
            "type": "FeatureCollection", "features": [{
                "type": "Feature", "properties": {}, "geometry": {
                    "type": "Polygon", "coordinates": [[[0,0],[1,0],[1,1],[0,1],[0,0]]]
                }
            }]
        }))
        .unwrap()
    }

    #[test]
    fn point_limit_defaults_and_overrides_are_shared_by_request_types() {
        for (value, expected) in [
            (json!(null), DEFAULT_POINT_LIMIT),
            (json!(0), 0),
            (json!(7_000_000), 7_000_000),
        ] {
            let args: Args = serde_json::from_value(json!({"point_limit": value})).unwrap();
            assert_eq!(args.init(None).point_limit, expected);
            let bounds: BoundsArg = serde_json::from_value(
                json!({"min_lat":0,"min_lon":0,"max_lat":1,"max_lon":1,"point_limit":value}),
            )
            .unwrap();
            assert_eq!(resolve_point_limit(bounds.point_limit), expected);
        }
        let omitted: Args = serde_json::from_value(json!({})).unwrap();
        assert_eq!(omitted.init(None).point_limit, 5_000_000);
        for invalid in [json!(-1), json!(1.5), json!("5000000")] {
            assert!(serde_json::from_value::<Args>(json!({"point_limit": invalid})).is_err());
        }
    }

    #[test]
    fn point_limit_counts_inside_points_and_stops_reading_at_the_cap() {
        let rows = vec![
            Ok(api::point_struct::PointStruct { lat: 2., lon: 2. }),
            Ok(api::point_struct::PointStruct { lat: 0.2, lon: 0.2 }),
            Ok(api::point_struct::PointStruct { lat: 0.3, lon: 0.3 }),
            Err(DbErr::Custom("must not read beyond limit".into())),
        ];
        let points = block_on(collect_in_area(stream::iter(rows), &area(), 2)).unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].lat, 0.2);
    }

    #[test]
    fn point_limit_zero_is_unlimited_and_stream_errors_propagate() {
        let rows = (0..10).map(|_| Ok(api::point_struct::PointStruct { lat: 0.5, lon: 0.5 }));
        assert_eq!(
            block_on(collect_in_area(stream::iter(rows), &area(), 0))
                .unwrap()
                .len(),
            10
        );
        let rows = vec![Err::<api::point_struct::PointStruct, _>(DbErr::Custom(
            "database failure".into(),
        ))];
        assert!(block_on(collect_in_area(stream::iter(rows), &area(), 0)).is_err());
    }

    #[test]
    fn point_limit_accepts_more_than_two_million_and_caps_at_five_million() {
        // All points are inside the fence. Keep this fixture compact while testing
        // the actual stream collector at the former limit and the new default.
        struct TestPoint;
        impl HasLatLon for TestPoint {
            fn lat(&self) -> f64 {
                0.5
            }
            fn lon(&self) -> f64 {
                0.5
            }
        }
        for count in [3_000_001, 5_000_001] {
            let rows = stream::iter((0..count).map(|_| Ok(TestPoint)));
            let points = block_on(collect_in_area(rows, &area(), DEFAULT_POINT_LIMIT)).unwrap();
            assert_eq!(points.len(), count.min(DEFAULT_POINT_LIMIT as usize));
        }
    }
}
