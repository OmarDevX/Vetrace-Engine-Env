use std::cmp::Reverse;
use std::collections::BinaryHeap;

use glam::{Vec2, Vec3};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridPoint {
    pub x: i32,
    pub z: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NavigationPath {
    pub points: Vec<Vec3>,
}

impl NavigationPath {
    pub fn is_empty(&self) -> bool { self.points.is_empty() }
}

#[derive(Clone, Debug)]
pub struct NavigationGrid {
    min: Vec2,
    width: usize,
    height: usize,
    cell_size: f32,
    allow_diagonal: bool,
    blocked: Vec<bool>,
}

impl NavigationGrid {
    pub fn new(min: Vec2, max: Vec2, cell_size: f32) -> Self {
        let cell_size = cell_size.max(0.05);
        let extent = (max - min).max(Vec2::splat(cell_size));
        let width = (extent.x / cell_size).ceil().max(1.0) as usize;
        let height = (extent.y / cell_size).ceil().max(1.0) as usize;
        Self { min, width, height, cell_size, allow_diagonal: true, blocked: vec![false; width * height] }
    }

    pub fn with_diagonal_movement(mut self, allow: bool) -> Self { self.allow_diagonal = allow; self }
    pub fn dimensions(&self) -> (usize, usize) { (self.width, self.height) }
    pub fn cell_size(&self) -> f32 { self.cell_size }

    pub fn world_to_cell(&self, world: Vec3) -> Option<GridPoint> {
        let relative = Vec2::new(world.x, world.z) - self.min;
        let point = GridPoint { x: (relative.x / self.cell_size).floor() as i32, z: (relative.y / self.cell_size).floor() as i32 };
        self.contains(point).then_some(point)
    }

    pub fn cell_center(&self, point: GridPoint, y: f32) -> Vec3 {
        Vec3::new(
            self.min.x + (point.x as f32 + 0.5) * self.cell_size,
            y,
            self.min.y + (point.z as f32 + 0.5) * self.cell_size,
        )
    }

    pub fn set_blocked(&mut self, point: GridPoint, blocked: bool) {
        if let Some(index) = self.index(point) { self.blocked[index] = blocked; }
    }

    pub fn is_walkable(&self, point: GridPoint) -> bool {
        self.index(point).map(|index| !self.blocked[index]).unwrap_or(false)
    }

    pub fn block_aabb(&mut self, center: Vec2, size: Vec2, clearance: f32) {
        let half = size.abs() * 0.5 + Vec2::splat(clearance.max(0.0));
        let min = center - half;
        let max = center + half;
        for z in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                let point = GridPoint { x, z };
                let cell = self.cell_center(point, 0.0);
                if cell.x >= min.x && cell.x <= max.x && cell.z >= min.y && cell.z <= max.y {
                    self.set_blocked(point, true);
                }
            }
        }
    }

    pub fn find_path(&self, start: Vec3, goal: Vec3) -> Option<NavigationPath> {
        let path_y = start.y;
        let start = self.nearest_walkable(self.world_to_cell(start)?)?;
        let goal = self.nearest_walkable(self.world_to_cell(goal)?)?;
        let start_index = self.index(start)?;
        let goal_index = self.index(goal)?;
        if start_index == goal_index { return Some(NavigationPath { points: vec![self.cell_center(goal, path_y)] }); }

        let mut frontier = BinaryHeap::new();
        let mut costs = vec![u32::MAX; self.blocked.len()];
        let mut came_from = vec![usize::MAX; self.blocked.len()];
        costs[start_index] = 0;
        frontier.push((Reverse(self.heuristic(start, goal)), start_index));

        while let Some((_, current_index)) = frontier.pop() {
            if current_index == goal_index { break; }
            let current = self.point(current_index);
            for (next, step_cost) in self.neighbors(current) {
                let next_index = self.index(next)?;
                let new_cost = costs[current_index].saturating_add(step_cost);
                if new_cost >= costs[next_index] { continue; }
                costs[next_index] = new_cost;
                came_from[next_index] = current_index;
                frontier.push((Reverse(new_cost.saturating_add(self.heuristic(next, goal))), next_index));
            }
        }
        if came_from[goal_index] == usize::MAX { return None; }

        let mut indices = vec![goal_index];
        let mut current = goal_index;
        while current != start_index {
            current = came_from[current];
            if current == usize::MAX { return None; }
            indices.push(current);
        }
        indices.reverse();
        let points = indices.into_iter().skip(1).map(|index| self.cell_center(self.point(index), path_y)).collect();
        Some(NavigationPath { points })
    }

    fn nearest_walkable(&self, origin: GridPoint) -> Option<GridPoint> {
        if self.is_walkable(origin) { return Some(origin); }
        let max_radius = self.width.max(self.height) as i32;
        for radius in 1..=max_radius {
            for z in (origin.z - radius)..=(origin.z + radius) {
                for x in (origin.x - radius)..=(origin.x + radius) {
                    if (x - origin.x).abs().max((z - origin.z).abs()) != radius { continue; }
                    let point = GridPoint { x, z };
                    if self.is_walkable(point) { return Some(point); }
                }
            }
        }
        None
    }

    fn neighbors(&self, point: GridPoint) -> Vec<(GridPoint, u32)> {
        let directions: &[(i32, i32, u32)] = if self.allow_diagonal {
            &[(1, 0, 10), (-1, 0, 10), (0, 1, 10), (0, -1, 10), (1, 1, 14), (1, -1, 14), (-1, 1, 14), (-1, -1, 14)]
        } else { &[(1, 0, 10), (-1, 0, 10), (0, 1, 10), (0, -1, 10)] };
        directions.iter().filter_map(|&(dx, dz, cost)| {
            let next = GridPoint { x: point.x.checked_add(dx)?, z: point.z.checked_add(dz)? };
            if !self.is_walkable(next) { return None; }
            if dx != 0 && dz != 0 {
                let horizontal = GridPoint { x: point.x.checked_add(dx)?, z: point.z };
                let vertical = GridPoint { x: point.x, z: point.z.checked_add(dz)? };
                if !self.is_walkable(horizontal) || !self.is_walkable(vertical) { return None; }
            }
            Some((next, cost))
        }).collect()
    }

    fn heuristic(&self, a: GridPoint, b: GridPoint) -> u32 {
        let dx = (a.x - b.x).unsigned_abs();
        let dz = (a.z - b.z).unsigned_abs();
        if self.allow_diagonal { 14 * dx.min(dz) + 10 * dx.abs_diff(dz) } else { 10 * (dx + dz) }
    }

    fn contains(&self, point: GridPoint) -> bool {
        let (Ok(x), Ok(z)) = (usize::try_from(point.x), usize::try_from(point.z)) else {
            return false;
        };
        x < self.width && z < self.height
    }

    fn index(&self, point: GridPoint) -> Option<usize> {
        let x = usize::try_from(point.x).ok()?;
        let z = usize::try_from(point.z).ok()?;
        if x >= self.width || z >= self.height {
            return None;
        }

        z.checked_mul(self.width)
            .and_then(|row| row.checked_add(x))
            .filter(|&index| index < self.blocked.len())
    }
    fn point(&self, index: usize) -> GridPoint { GridPoint { x: (index % self.width) as i32, z: (index / self.width) as i32 } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_routes_around_a_wall() {
        let mut grid = NavigationGrid::new(Vec2::ZERO, Vec2::splat(10.0), 1.0);
        grid.block_aabb(Vec2::new(5.0, 5.0), Vec2::new(1.0, 7.0), 0.0);
        let path = grid.find_path(Vec3::new(1.0, 0.0, 5.0), Vec3::new(9.0, 0.0, 5.0)).expect("route around wall");
        assert!(!path.is_empty());
        assert!(path.points.iter().all(|point| grid.world_to_cell(*point).map(|cell| grid.is_walkable(cell)).unwrap_or(false)));
    }

    #[test]
    fn diagonal_paths_do_not_cut_blocked_corners() {
        let mut grid = NavigationGrid::new(Vec2::ZERO, Vec2::splat(3.0), 1.0);
        grid.set_blocked(GridPoint { x: 1, z: 0 }, true);
        grid.set_blocked(GridPoint { x: 0, z: 1 }, true);
        assert!(grid.find_path(Vec3::new(0.5, 0.0, 0.5), Vec3::new(2.5, 0.0, 2.5)).is_none());
    }

    #[test]
    fn out_of_bounds_points_do_not_overflow_index_calculation() {
        let grid = NavigationGrid::new(Vec2::ZERO, Vec2::splat(3.0), 1.0);
        assert_eq!(grid.index(GridPoint { x: -1, z: 0 }), None);
        assert_eq!(grid.index(GridPoint { x: 0, z: -1 }), None);
        assert_eq!(grid.index(GridPoint { x: 3, z: 0 }), None);
        assert_eq!(grid.index(GridPoint { x: 0, z: 3 }), None);
    }

    #[test]
    fn path_from_grid_boundary_handles_out_of_bounds_neighbors() {
        let grid = NavigationGrid::new(Vec2::ZERO, Vec2::splat(3.0), 1.0);
        let path = grid
            .find_path(Vec3::new(0.5, 0.0, 0.5), Vec3::new(2.5, 0.0, 2.5))
            .expect("path from boundary cell");
        assert!(!path.is_empty());
    }
}
