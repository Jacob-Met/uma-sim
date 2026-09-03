//! Half-open intervals [start, end) on the course.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Region {
    pub start: f64,
    pub end: f64,
}

impl Region {
    pub fn new(start: f64, end: f64) -> Self {
        Self { start, end }
    }

    pub fn empty() -> Self {
        Self {
            start: -1.0,
            end: -1.0,
        }
    }

    pub fn is_empty(self) -> bool {
        self.end <= self.start || self.start < 0.0
    }

    pub fn len(self) -> f64 {
        if self.is_empty() {
            0.0
        } else {
            self.end - self.start
        }
    }

    pub fn intersect(self, other: Region) -> Region {
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        if end <= start {
            Region::empty()
        } else {
            Region { start, end }
        }
    }

    pub fn contains(self, pos: f64) -> bool {
        !self.is_empty() && pos >= self.start && pos < self.end
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RegionList {
    pub regions: Vec<Region>,
}

impl RegionList {
    pub fn whole_course(distance: f64) -> Self {
        Self {
            regions: vec![Region::new(0.0, distance)],
        }
    }

    pub fn push(&mut self, r: Region) {
        if !r.is_empty() {
            self.regions.push(r);
        }
    }

    pub fn map_intersect(&self, bounds: Region) -> Self {
        let mut out = RegionList::default();
        for r in &self.regions {
            out.push(r.intersect(bounds));
        }
        out
    }

    /// Intersect each region with every bounds segment (oracle `rmap` over slope pieces).
    pub fn map_intersect_all(&self, bounds: &[Region]) -> Self {
        let mut out = RegionList::default();
        for r in &self.regions {
            for b in bounds {
                out.push(r.intersect(*b));
            }
        }
        out
    }

    /// Pairwise intersect two region lists (precondition ∩ condition).
    pub fn intersect_list(&self, other: &RegionList) -> Self {
        let mut out = RegionList::default();
        for a in &self.regions {
            for b in &other.regions {
                out.push(a.intersect(*b));
            }
        }
        out
    }

    pub fn total_len(&self) -> f64 {
        self.regions.iter().map(|r| r.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    /// Merge overlapping/adjacent halves (umalator `RegionList.union`).
    pub fn union(&self, other: &RegionList) -> RegionList {
        let mut u: Vec<Region> = self.regions.clone();
        u.extend(other.regions.iter().copied());
        if u.is_empty() {
            return RegionList::default();
        }
        u.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
        let mut out = RegionList::default();
        let mut cur = u[0];
        for b in u.into_iter().skip(1) {
            if cur.start <= b.start && cur.end >= b.end {
                // fully contains
                continue;
            } else if cur.start <= b.start && b.start < cur.end {
                cur = Region::new(cur.start, b.end);
            } else if cur.start < b.end && b.end <= cur.end {
                cur = Region::new(b.start, cur.end);
            } else {
                out.push(cur);
                cur = b;
            }
        }
        out.push(cur);
        out
    }
}
