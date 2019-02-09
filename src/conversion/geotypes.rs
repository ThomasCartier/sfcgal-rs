use failure::Error;
#[allow(unused_imports)]
use sfcgal_sys::{
    sfcgal_geometry_t,
    sfcgal_point_create_from_xy, sfcgal_point_x, sfcgal_point_y, sfcgal_point_z,
    sfcgal_linestring_create, sfcgal_linestring_add_point, sfcgal_linestring_point_n, sfcgal_linestring_num_points,
    sfcgal_triangle_create, sfcgal_triangle_set_vertex_from_xy, sfcgal_triangle_vertex,
    sfcgal_polygon_create, sfcgal_polygon_create_from_exterior_ring, sfcgal_polygon_add_interior_ring,
    sfcgal_polygon_num_interior_rings, sfcgal_polygon_exterior_ring, sfcgal_polygon_interior_ring_n,
    sfcgal_multi_point_create, sfcgal_multi_linestring_create, sfcgal_multi_polygon_create,
    sfcgal_geometry_collection_add_geometry, sfcgal_geometry_collection_num_geometries,
    sfcgal_geometry_collection_geometry_n, sfcgal_geometry_collection_create,
};
use crate::{Result, SFCGeometry, GeomType, ToSFCGAL, utils::check_null_geom};

/// Conversion from [`SFCGeometry`] (implemented on [geo-types](https://docs.rs/geo-types/) geometries)
///
/// [`SFCGeometry`]: struct.SFCGeometry.html
pub trait TryInto<T> {
    type Err;
    fn try_into(self) -> Result<T>;
}

impl TryInto<geo_types::Geometry<f64>> for SFCGeometry {
    type Err = Error;

    fn try_into(self) -> Result<geo_types::Geometry<f64>> {
        match self._type()? {
            GeomType::Point => {
                Ok(
                    geo_types::Geometry::Point(
                        unsafe { geo_point_from_sfcgal(self.c_geom.as_ref()) }
                    )
                )
            },
            GeomType::Multipoint => {
                let ngeoms = unsafe {
                    sfcgal_geometry_collection_num_geometries(self.c_geom.as_ref())
                };
                let mut pts = Vec::with_capacity(ngeoms);
                for i in 0..ngeoms {
                    let geom = unsafe { sfcgal_geometry_collection_geometry_n(self.c_geom.as_ref(), i) };
                    pts.push(geo_point_from_sfcgal(geom));
                }
                Ok(
                    geo_types::Geometry::MultiPoint(
                        geo_types::MultiPoint(pts)
                    )
                )
            },
            GeomType::Linestring => {
                Ok(
                    geo_types::Geometry::LineString(
                        geo_line_from_sfcgal(unsafe { self.c_geom.as_ref() })?
                    )
                )
            },
            GeomType::Multilinestring => {
                let ngeoms = unsafe {
                    sfcgal_geometry_collection_num_geometries(self.c_geom.as_ref())
                };
                let mut lines = Vec::with_capacity(ngeoms);
                for i in 0..ngeoms {
                    let geom = unsafe { sfcgal_geometry_collection_geometry_n(self.c_geom.as_ref(), i) };
                    lines.push(geo_line_from_sfcgal(geom)?);
                }
                Ok(
                    geo_types::Geometry::MultiLineString(
                        geo_types::MultiLineString(lines)
                    )
                )
            },
            GeomType::Polygon => {
                let nrings = unsafe { sfcgal_polygon_num_interior_rings(self.c_geom.as_ref()) };
                let exterior_sfcgal = unsafe { sfcgal_polygon_exterior_ring(self.c_geom.as_ref()) };
                let exterior_geo = geo_line_from_sfcgal(exterior_sfcgal)?;
                let mut interiors_geo = Vec::with_capacity(nrings);
                for i in 0..nrings {
                    let line_sfcgal = unsafe {
                        sfcgal_polygon_interior_ring_n(self.c_geom.as_ref(), i)
                    };
                    interiors_geo.push(geo_line_from_sfcgal(line_sfcgal)?);
                }

                Ok(
                    geo_types::Geometry::Polygon(
                        geo_types::Polygon::new(exterior_geo, interiors_geo)
                    )
                )
            }
            GeomType::Multipolygon => {
                let ngeoms = unsafe {
                    sfcgal_geometry_collection_num_geometries(self.c_geom.as_ref())
                };
                let mut vec_polygons = Vec::with_capacity(ngeoms);
                for i in 0..ngeoms {
                    let _polyg = unsafe { sfcgal_geometry_collection_geometry_n(self.c_geom.as_ref(), i) };
                    let nrings = unsafe { sfcgal_polygon_num_interior_rings(_polyg) };
                    let exterior_sfcgal = unsafe { sfcgal_polygon_exterior_ring(_polyg) };
                    let exterior_geo = geo_line_from_sfcgal(exterior_sfcgal)?;
                    let mut interiors_geo = Vec::with_capacity(nrings);
                    for j in 0..nrings {
                        let line_sfcgal = unsafe {
                            sfcgal_polygon_interior_ring_n(_polyg, j)
                        };
                        interiors_geo.push(geo_line_from_sfcgal(line_sfcgal)?);
                    }
                    vec_polygons.push(geo_types::Polygon::new(exterior_geo, interiors_geo));
                }

                Ok(
                    geo_types::Geometry::MultiPolygon(
                        geo_types::MultiPolygon(vec_polygons)
                    )
                )
            }
            _ => unimplemented!()
        }
    }
}


fn geo_line_from_sfcgal(sfcgal_geom: *const sfcgal_geometry_t) -> Result<geo_types::LineString<f64>> {
    let n_points = unsafe { sfcgal_linestring_num_points(sfcgal_geom) };
    let mut v_points = Vec::with_capacity(n_points);
    for i in 0..n_points {
        let pt_sfcgal = unsafe { sfcgal_linestring_point_n(sfcgal_geom, i) };
        check_null_geom(pt_sfcgal)?;
        let pt_geom = geo_point_from_sfcgal(pt_sfcgal);
        v_points.push(pt_geom);
    }
    Ok(geo_types::LineString::from(v_points))
}
fn geo_point_from_sfcgal(geom: *const sfcgal_geometry_t) -> geo_types::Point<f64> {
    let x = unsafe { sfcgal_point_x(geom) };
    let y = unsafe { sfcgal_point_y(geom) };
    geo_types::Point::new(x, y)
}

/// Create a `SFCGeometry` from a geo-types Point
impl ToSFCGAL for geo_types::Point<f64> {
    fn to_sfcgal(&self) -> Result<SFCGeometry> {
        let geom = unsafe { sfcgal_point_create_from_xy(self.x(), self.y()) };
        unsafe { SFCGeometry::new_from_raw(geom, true) }
    }
}

/// Create a `SFCGeometry` from a geo-types MultiPoint
impl ToSFCGAL for geo_types::MultiPoint<f64> {
    fn to_sfcgal(&self) -> Result<SFCGeometry> {
        let out_geom = unsafe { sfcgal_multi_point_create() };
        check_null_geom(out_geom)?;
        let &geo_types::MultiPoint(ref point_list) = self;
        for point in point_list.iter() {
            let geom = unsafe {
                sfcgal_point_create_from_xy(point.x(), point.y())
            };
            check_null_geom(geom)?;
            unsafe {
                sfcgal_geometry_collection_add_geometry(out_geom, geom)
            };
        }
        unsafe { SFCGeometry::new_from_raw(out_geom, true) }
    }
}

/// Create a `SFCGeometry` from a geo-types Line
impl ToSFCGAL for geo_types::Line<f64> {
    fn to_sfcgal(&self) -> Result<SFCGeometry> {
        let out_linestring = unsafe { sfcgal_linestring_create() };
        check_null_geom(out_linestring)?;
        let start = unsafe {
            sfcgal_point_create_from_xy(self.start.x, self.start.y)
        };
        let end = unsafe {
            sfcgal_point_create_from_xy(self.end.x, self.end.y)
        };
        check_null_geom(start)?;
        check_null_geom(end)?;
        unsafe {
            sfcgal_linestring_add_point(out_linestring, start);
            sfcgal_linestring_add_point(out_linestring, end);
            SFCGeometry::new_from_raw(out_linestring, true)
        }
    }
}
/// Create a `SFCGeometry` from a geo-types LineString
impl ToSFCGAL for geo_types::LineString<f64> {
    fn to_sfcgal(&self) -> Result<SFCGeometry> {
        unsafe { SFCGeometry::new_from_raw(linestring_geo_to_sfcgal(self)?, true) }
    }
}

/// Create a `SFCGeometry` from a geo-types MultiLineString
impl ToSFCGAL for geo_types::MultiLineString<f64> {
    fn to_sfcgal(&self) -> Result<SFCGeometry> {
        let out_multilinestring = unsafe { sfcgal_multi_linestring_create() };
        check_null_geom(out_multilinestring)?;
        let &geo_types::MultiLineString(ref linestring_list) = self;
        for _linestring in linestring_list.into_iter() {
            let out_sfcgal_line = linestring_geo_to_sfcgal(_linestring)?;
            unsafe {
                sfcgal_geometry_collection_add_geometry(out_multilinestring, out_sfcgal_line)
            };
        }
        unsafe { SFCGeometry::new_from_raw(out_multilinestring, true) }
    }
}

/// Create a `SFCGeometry` from a geo-types Triangle
impl ToSFCGAL for geo_types::Triangle<f64> {
    fn to_sfcgal(&self) -> Result<SFCGeometry> {
        let out_triangle = unsafe { sfcgal_triangle_create() };
        check_null_geom(out_triangle)?;
        let &geo_types::Triangle(ref c0, ref c1, ref c2) = self;
        unsafe {
            sfcgal_triangle_set_vertex_from_xy(out_triangle, 0, c0.x, c0.y);
            sfcgal_triangle_set_vertex_from_xy(out_triangle, 1, c1.x, c1.y);
            sfcgal_triangle_set_vertex_from_xy(out_triangle, 2, c2.x, c2.y);
            SFCGeometry::new_from_raw(out_triangle, true)
        }
    }
}

/// Create a `SFCGeometry` from a geo-types Polygon
impl ToSFCGAL for geo_types::Polygon<f64> {
    fn to_sfcgal(&self) -> Result<SFCGeometry> {
        let &geo_types::Polygon{ref exterior, ref interiors} = self;

        let out_polygon = unsafe {
            sfcgal_polygon_create_from_exterior_ring(linestring_geo_to_sfcgal(exterior)?)
        };
        check_null_geom(out_polygon)?;

        for ring in interiors {
            unsafe {
                sfcgal_polygon_add_interior_ring(out_polygon, linestring_geo_to_sfcgal(ring)?)
            };
        }
        unsafe { SFCGeometry::new_from_raw(out_polygon, true) }
    }
}

/// Create a `SFCGeometry` from a geo-types MultiPolygon
impl ToSFCGAL for geo_types::MultiPolygon<f64> {
    fn to_sfcgal(&self) -> Result<SFCGeometry> {
        let out_multipolygon = unsafe { sfcgal_multi_polygon_create() };
        let &geo_types::MultiPolygon(ref list_polygons) = self;
        for polygon in list_polygons {
            let &geo_types::Polygon{ref exterior, ref interiors} = polygon;
            let out_polygon = unsafe {
                sfcgal_polygon_create_from_exterior_ring(linestring_geo_to_sfcgal(exterior)?)
            };
            check_null_geom(out_polygon)?;

            for ring in interiors {
                unsafe {
                    sfcgal_polygon_add_interior_ring(out_polygon, linestring_geo_to_sfcgal(ring)?)
                };
            }
            unsafe {
                sfcgal_geometry_collection_add_geometry(out_multipolygon, out_polygon)
            };
        }
        unsafe { SFCGeometry::new_from_raw(out_multipolygon, true) }
    }
}

/// Create a `SFCGeometry` from a geo-types GeometryCollection
impl ToSFCGAL for geo_types::GeometryCollection<f64> {
    fn to_sfcgal(&self) -> Result<SFCGeometry> {
        let out_geom_collection = unsafe { sfcgal_geometry_collection_create() };
        let &geo_types::GeometryCollection(ref list_geoms) = self;
        for g_geom in list_geoms {
            let sfcgal_geom = g_geom.to_sfcgal()?;
            unsafe {
                sfcgal_geometry_collection_add_geometry(out_geom_collection, sfcgal_geom.c_geom.as_ptr())
            };
        }
        unsafe { SFCGeometry::new_from_raw(out_geom_collection, true) }
    }
}

/// Create a `SFCGeometry` from any geo-type Geometry
impl ToSFCGAL for geo_types::Geometry<f64> {
    fn to_sfcgal(&self) -> Result<SFCGeometry> {
        match *self {
            geo_types::Geometry::Point(ref c) => c.to_sfcgal(),
            geo_types::Geometry::Line(ref c) => c.to_sfcgal(),
            geo_types::Geometry::LineString(ref c) => c.to_sfcgal(),
            geo_types::Geometry::Polygon(ref c) => c.to_sfcgal(),
            geo_types::Geometry::MultiPoint(ref c) => c.to_sfcgal(),
            geo_types::Geometry::MultiLineString(ref c) => c.to_sfcgal(),
            geo_types::Geometry::MultiPolygon(ref c) => c.to_sfcgal(),
            geo_types::Geometry::GeometryCollection(ref c) => c.to_sfcgal()
        }
    }
}

fn linestring_geo_to_sfcgal(geom: &geo_types::LineString<f64>) -> Result<*mut sfcgal_geometry_t> {
    let out_linestring = unsafe { sfcgal_linestring_create() };
    check_null_geom(out_linestring)?;
    let &geo_types::LineString(ref point_list) = geom;
    for coords in point_list.iter() {
        let geom = unsafe {
            sfcgal_point_create_from_xy(coords.x, coords.y)
        };
        check_null_geom(geom)?;
        unsafe {
            sfcgal_linestring_add_point(out_linestring, geom)
        };
    }
    Ok(out_linestring)
}


#[cfg(test)]
mod tests {
    use geo_types::{
        Coordinate, Point, MultiPoint,
        LineString, MultiLineString, Polygon, MultiPolygon, Triangle
    };
    use crate::{SFCGeometry, ToSFCGAL, GeomType};
    use super::TryInto;

    #[test]
    fn point_geo_to_sfcgal_to_geo() {
        let pt = Point::new(0.1, 0.9);
        let pt_sfcgal = pt.to_sfcgal().unwrap();
        assert!(pt_sfcgal.is_valid().unwrap());
        let pt: Point<f64> = pt_sfcgal.try_into().unwrap().as_point().unwrap();
        assert_eq!(pt.x(), 0.1);
        assert_eq!(pt.y(), 0.9);
    }

    #[test]
    fn point_sfcgal_try_into_geo() {
        let pt_sfcgal = SFCGeometry::new("POINT(0.1 0.9)").unwrap();
        let pt: Point<f64> = pt_sfcgal.try_into().unwrap().as_point().unwrap();
        assert_ulps_eq!(pt.x(), 0.1);
        assert_ulps_eq!(pt.y(), 0.9);
    }

    #[test]
    fn multipoint_geo_to_sfcgal_to_geo() {
        let multipt = MultiPoint::from(vec![
            Point::new(0., 0.),
            Point::new(1., 1.),
        ]);
        let mpt_sfcgal = multipt.to_sfcgal().unwrap();
        assert!(mpt_sfcgal.is_valid().unwrap());
        let mpt: MultiPoint<f64> = mpt_sfcgal.try_into().unwrap().as_multipoint().unwrap();
        assert_eq!(mpt.0[0].x(), 0.);
        assert_eq!(mpt.0[0].y(), 0.);
        assert_eq!(mpt.0[1].x(), 1.);
        assert_eq!(mpt.0[1].y(), 1.);
    }

    #[test]
    fn linestring_geo_to_sfcgal_to_geo() {
        let linestring = LineString::from(vec![
            Point::new(0., 0.),
            Point::new(1., 1.),
        ]);
        let line_sfcgal = linestring.to_sfcgal().unwrap();
        assert!(line_sfcgal.is_valid().unwrap());
        let linestring_geo: LineString<f64> = line_sfcgal.try_into().unwrap().as_linestring().unwrap();
        assert_eq!(linestring_geo.0[0].x, 0.);
        assert_eq!(linestring_geo.0[0].y, 0.);
        assert_eq!(linestring_geo.0[1].x, 1.);
        assert_eq!(linestring_geo.0[1].y, 1.);
    }

    #[test]
    fn multilinestring_geo_to_sfcgal_to_geo() {
        let multilinestring = MultiLineString::from(LineString::from(vec![
            Point::new(0., 0.),
            Point::new(1., 1.),
        ]));
        let mls_sfcgal = multilinestring.to_sfcgal().unwrap();
        assert!(mls_sfcgal.is_valid().unwrap());
        let mls: MultiLineString<f64> = mls_sfcgal.try_into().unwrap().as_multilinestring().unwrap();
        assert_eq!(mls.0[0].0[0].x, 0.);
        assert_eq!(mls.0[0].0[0].y, 0.);
        assert_eq!(mls.0[0].0[1].x, 1.);
        assert_eq!(mls.0[0].0[1].y, 1.);
    }

    #[test]
    fn triangle_geo_to_sfcgal() {
        let tri = Triangle(
            Coordinate::from((0., 0.)),
            Coordinate::from((1., 0.)),
            Coordinate::from((0.5, 1.)),
        );
        let tri_sfcgal = tri.to_sfcgal().unwrap();
        assert!(tri_sfcgal.is_valid().unwrap());
        assert_eq!(tri_sfcgal._type().unwrap(), GeomType::Triangle);
    }


    #[test]
    fn polygon_geo_to_sfcgal_to_geo() {
        let polygon = Polygon::new(
            LineString::from(vec![(0., 0.), (1., 0.), (1., 1.), (0., 1.,), (0., 0.)]),
            vec![LineString::from(
                vec![(0.1, 0.1), (0.1, 0.9,), (0.9, 0.9), (0.9, 0.1), (0.1, 0.1)])]);
        let poly_sfcgal = polygon.to_sfcgal().unwrap();
        let polyg: Polygon<f64> = poly_sfcgal.try_into().unwrap().as_polygon().unwrap();

        assert_eq!(
            polyg.exterior,
            LineString::from(vec![(0., 0.), (1., 0.), (1., 1.), (0., 1.,), (0., 0.)]));
        assert_eq!(polyg.interiors[0].0[0].x, 0.1);
        assert_eq!(polyg.interiors[0].0[0].y, 0.1);
        assert_eq!(polyg.interiors[0].0[2].x, 0.9);
        assert_eq!(polyg.interiors[0].0[2].y, 0.9);
        assert_eq!(polyg.interiors[0].0[3].x, 0.9);
        assert_eq!(polyg.interiors[0].0[3].y, 0.1);
    }

    #[test]
    fn multipolygon_geo_to_sfcgal_to_geo() {
        let multipolygon = MultiPolygon(vec![
            Polygon::new(
                LineString::from(vec![(0., 0.), (1., 0.), (1., 1.), (0., 1.,), (0., 0.)]),
                vec![LineString::from(
                    vec![(0.1, 0.1), (0.1, 0.9,), (0.9, 0.9), (0.9, 0.1), (0.1, 0.1)])]
            ),
        ]);
        let mutlipolygon_sfcgal = multipolygon.to_sfcgal().unwrap();
        let mpg: MultiPolygon<f64> = mutlipolygon_sfcgal.try_into().unwrap().as_multipolygon().unwrap();

        assert_eq!(
            mpg.0[0].exterior,
            LineString::from(vec![(0., 0.), (1., 0.), (1., 1.), (0., 1.,), (0., 0.)]));
        assert_eq!(
            mpg.0[0].interiors[0],
            LineString::from(
                vec![(0.1, 0.1), (0.1, 0.9,), (0.9, 0.9), (0.9, 0.1), (0.1, 0.1)]));
    }
}