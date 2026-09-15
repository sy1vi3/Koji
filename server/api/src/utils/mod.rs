use super::*;

use geo::Point;
use geojson::{Geometry, Value};
use model::{
    KojiDb, ScannerType,
    api::{
        BBox, ToCollection,
        args::{ApiQueryArgs, SpawnpointTth, UnknownId},
        single_vec::SingleVec,
    },
    db::{GenericData, area, geofence, gym, instance, pokestop, spawnpoint, station},
    error::ModelError,
};

pub mod auth;
pub mod error;
pub mod request;
pub mod response;

pub fn is_docker() -> io::Result<bool> {
    let mut path = env::current_dir()?;
    path.push("dist");
    let metadata = fs::metadata(path)?;
    Ok(metadata.is_dir())
}

pub async fn load_collection(
    instance: &String,
    conn: &KojiDb,
) -> Result<FeatureCollection, ModelError> {
    match load_feature(instance, conn).await {
        Ok(feature) => Ok(feature.to_collection(None, None)),
        Err(err) => Err(err),
    }
}

pub async fn load_feature(instance: &String, conn: &KojiDb) -> Result<Feature, ModelError> {
    match geofence::Query::get_one_feature(
        &conn.koji,
        instance.to_string(),
        &ApiQueryArgs::default(),
    )
    .await
    {
        Ok(area) => Ok(area),
        Err(_) => {
            if conn.scanner_type == ScannerType::Unown {
                area::Query::feature_from_name(
                    &conn.controller,
                    &instance,
                    "auto_quest".to_string(),
                )
                .await
            } else {
                instance::Query::feature_from_name(&conn.controller, &instance).await
            }
        }
    }
}

pub async fn create_or_find_collection(
    instance: &String,
    conn: &KojiDb,
    area: FeatureCollection,
    parent: &Option<UnknownId>,
    data_points: &SingleVec,
) -> Result<FeatureCollection, ModelError> {
    if !data_points.is_empty() {
        let bbox = BBox::new(&data_points.iter().map(|p| Point::new(p[1], p[0])).collect());
        Ok(FeatureCollection {
            bbox: bbox.get_geojson_bbox(),
            features: vec![Feature {
                bbox: bbox.get_geojson_bbox(),
                geometry: Some(Geometry {
                    value: Value::Polygon(bbox.get_poly()),
                    bbox: None,
                    foreign_members: None,
                }),
                ..Feature::default()
            }],
            ..FeatureCollection::default()
        })
    } else if !area.features.is_empty() {
        Ok(area)
    } else if let Some(parent) = parent {
        geofence::Query::by_parent(&conn.koji, parent).await
    } else if !instance.is_empty() {
        load_collection(instance, conn).await
    } else {
        Ok(FeatureCollection::default())
    }
}

pub async fn points_from_area(
    area: &FeatureCollection,
    category: &String,
    conn: &KojiDb,
    last_seen: u32,
    tth: SpawnpointTth,
    point_limit: u64,
) -> Result<Vec<GenericData>, DbErr> {
    if !area.features.is_empty() {
        match category.as_str() {
            "gym" => gym::Query::area(&conn.scanner, &area, last_seen, point_limit).await,
            "pokestop" => pokestop::Query::area(&conn.scanner, &area, last_seen, point_limit).await,
            "station" => station::Query::area(&conn.scanner, &area, last_seen, point_limit).await,
            "spawnpoint" => {
                spawnpoint::Query::area(&conn.scanner, &area, last_seen, tth, point_limit).await
            }
            "fort" => {
                // Share one budget across categories instead of returning up to 3x the limit.
                let mut points =
                    gym::Query::area(&conn.scanner, &area, last_seen, point_limit).await?;
                if point_limit != 0 && points.len() as u64 >= point_limit {
                    return Ok(points);
                }
                let remaining = if point_limit == 0 {
                    0
                } else {
                    point_limit - points.len() as u64
                };
                points.extend(
                    pokestop::Query::area(&conn.scanner, &area, last_seen, remaining).await?,
                );
                if point_limit != 0 && points.len() as u64 >= point_limit {
                    return Ok(points);
                }
                let remaining = if point_limit == 0 {
                    0
                } else {
                    point_limit - points.len() as u64
                };
                points.extend(
                    station::Query::area(&conn.scanner, &area, last_seen, remaining).await?,
                );
                Ok(points)
            }
            _ => Err(DbErr::Custom("Invalid Category".to_string())),
        }
    } else {
        Ok(vec![])
    }
}
