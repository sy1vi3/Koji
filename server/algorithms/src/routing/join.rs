use crate::plugin::Plugin;
use crate::utils;
use geo::{Distance, Haversine, Point};
use model::api::{point_array::PointArray, single_vec::SingleVec};
use s2::cellid::CellID;
use s2::latlng::LatLng;
use std::collections::HashMap;
use std::io;
use std::time::Instant;

pub fn join(plugin: &Plugin, input: Vec<SingleVec>) -> io::Result<SingleVec> {
    if plugin.split_level == 0 || input.len() < 3 {
        return Ok(input.into_iter().flatten().collect());
    }
    let time = Instant::now();
    let mut point_map = HashMap::<u64, SingleVec>::new();

    let get_cell_id = |point: PointArray| {
        CellID::from(LatLng::from_degrees(point[0], point[1]))
            .parent(plugin.split_level)
            .0
    };

    let mut centroids = vec![];
    for points in input {
        if points.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Cannot join an empty route",
            ));
        }
        let center = utils::centroid(&points);
        centroids.push(center);
        if point_map.insert(get_cell_id(center), points).is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Route centroids share an S2 cell",
            ));
        }
    }
    // Solve the order of ALL chunks in one pass. Splitting these centroids
    // again produces singleton inputs instead of a route between chunks.
    let ordered_centroids = plugin.run(utils::stringify_points(&centroids))?;
    let mut clusters = Vec::with_capacity(centroids.len());
    for center in ordered_centroids {
        let points = point_map.remove(&get_cell_id(center)).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Solver returned an unknown or repeated route centroid",
            )
        })?;
        clusters.push(points);
    }
    if !point_map.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Solver omitted route centroids",
        ));
    }

    let mut final_routes: SingleVec = vec![];

    let last = clusters.len() - 1;
    for (i, current) in clusters.clone().iter_mut().enumerate() {
        let next: &SingleVec = if i == last {
            clusters[0].as_ref()
        } else {
            clusters[i + 1].as_ref()
        };

        let mut shortest = std::f64::MAX;
        let mut shortest_current_index = 0;

        for (current_index, current_point) in current.iter().enumerate() {
            let current_point = Point::new(current_point[1], current_point[0]);
            for (_next_index, next_point) in next.iter().enumerate() {
                let next_point = Point::new(next_point[1], next_point[0]);
                let distance = Haversine.distance(current_point, next_point);
                if distance < shortest {
                    shortest = distance;
                    shortest_current_index = current_index;
                }
            }
        }
        current.rotate_left(shortest_current_index);
        final_routes.append(current);
    }
    log::info!(
        "joined {} routes in {}ms",
        final_routes.len(),
        time.elapsed().as_millis()
    );
    Ok(final_routes)
}
