type Index = usize;
type Num = f32;
type Point = (Num, Num);

pub struct KDBush {
    ids: Vec<Index>,
    points: Vec<(Num, Num)>,
    node_size: u8,
}

impl KDBush {
    pub fn new(pts) -> Self {
        let ids = Vec::with_capacity(cap);
        let points = Vec::with_capacity(cap);

        Self { ids, points, node_size }
    }
}



function defaultGetX(p) { return p[0]; }
function defaultGetY(p) { return p[1]; }

export default function kdbush(points, getX, getY, nodeSize, ArrayType) {
    return new KDBush(points, getX, getY, nodeSize, ArrayType);
}

function KDBush(points, getX, getY, nodeSize, ArrayType) {
    getX = getX || defaultGetX;
    getY = getY || defaultGetY;
    ArrayType = ArrayType || Array;

    this.nodeSize = nodeSize || 64;

    let mut ids = Vec::with_capacity(points.len());
    let mut coords: Vec<(f32, f32)> = Vec::with_capacity(points.len());

    for (i, pt) in points.iter().enumerate() {
        ids.push(i);
        coords.push(pt.into());
    }

    sort(this.ids, this.coords, this.nodeSize, 0, this.ids.length - 1, 0);
}

impl KDBush {
    pub fn range(&self, min: Point, max: Point, visitor: F)
        where F: FnMut(Index)
    {
        self.range_idx(min, max, &mut visitor, 0, self.ids.len() - 1, 0);
    }

    pub fn within(&self, center: Point, radius: Num, visitor: F)
        where F: FnMut(Index)
    {
        self.within_idx(center, radius, &mut visitor, 0, self.ids.len() - 1, 0);
    }
}

fn range(ids, coords, min, max, nodeSize) {
    let stack = [0, ids.length - 1, 0];
    let result = [];
    let x, y;

    while !stack.is_empty() {
        let axis = stack.pop();
        let right = stack.pop();
        let left = stack.pop();

        if right - left <= node_size {
            for i in left..=right {
                let x = coords[i].0;
                let y = coords[i].1;
                if x >= minX && x <= maxX && y >= minY && y <= maxY {
                    result.push(ids[i]);
                }
            }
            continue;
        }

        let m = Math.floor((left + right) / 2).floor();

        let x = coords[2 * m];
        let y = coords[2 * m + 1];

        if x >= minX && x <= maxX && y >= minY && y <= maxY {
            result.push(ids[m]);
        }

        let next_axis = (axis + 1) % 2;

        if (axis === 0 ? minX <= x : minY <= y) {
            stack.push(left);
            stack.push(m - 1);
            stack.push(next_axis);
        }
        if (axis === 0 ? maxX >= x : maxY >= y) {
            stack.push(m + 1);
            stack.push(right);
            stack.push(next_axis);
        }
    }

    result
}

fn sort_kd(&mut self, left: Index, right: Index, axis: u8) {
    if right - left <= self.node_size as Index {
        return;
    }

    let middle: Index = (left + right) / 2;
    self.select(middle, left, right, axis);

    let next_axis = (axis + 1) % 2;
    self.sort_kd(left, middle - 1, next_axis);
    self.sort_kd(middle + 1, right, next_axis);
}

fn select(ids, coords, k, left, right, inc) {
    while right > left {
        if right - left > 600 {
            let n = right - left + 1;
            let m = k - left + 1;
            let z = Math.log(n);
            let s = 0.5 * Math.exp(2 * z / 3);
            let sd = 0.5 * Math.sqrt(z * s * (n - s) / n) * (m - n / 2 < 0 ? -1 : 1);
            let newLeft = Math.max(left, Math.floor(k - m * s / n + sd));
            let newRight = Math.min(right, Math.floor(k + (n - m) * s / n + sd));
            self.select(ids, coords, k, newLeft, newRight, inc);
        }

        let t = coords[2 * k + inc];
        let i = left;
        let j = right;

        swap_item(ids, coords, left, k);
        if coords[2 * right + inc] > t {
            swapItem(ids, coords, left, right);
        }

        while i < j {
            swap_item(ids, coords, i, j);
            i += 1;
            j -= 1;
            while coords[2 * i + inc] < t { i += 1 };
            while coords[2 * j + inc] > t { j -= 1 };
        }

        if (coords[2 * left + inc] === t) {
            swap_item(ids, coords, left, j);
        } else {
            j += 1;
            swap_item(ids, coords, j, right);
        }

        if j <= k { left = j + 1; }
        if k <= j { right = j - 1; }
    }
}

    fn swap_item(&mut self, i: Index, j: Index) {
        self.ids.swap(i, j)
        self.points.swap(i, j)
    }

fn within(ids, coords, qx, qy, r, nodeSize) {
    let stack = [0, ids.length - 1, 0];
    let result = [];
    let r2 = r * r;

    while !stack.is_empty() {
        let axis = stack.pop();
        let right = stack.pop();
        let left = stack.pop();

        if right - left <= node_size {
            for i in left..=right {
                if sq_dist(coords[i].0, coords[i].1, qx, qy) <= r2 {
                    result.push(ids[i]);
                }
            }
            continue;
        }

        let m = ((left + right) / 2.0).floor() as usize;

        let x = coords[m].0;
        let y = coords[m].1;

        if sq_dist(x, y, qx, qy) <= r2 {
            result.push(ids[m])
        }

        let next_axis = (axis + 1) % 2;

        if if axis == 0 { qx - r <= x } else { qy - r <= y } {
            stack.push(left);
            stack.push(m - 1);
            stack.push(next_axis);
        }
        if if axis == 0 { qx + r >= x } else { qy + r >= y } {
            stack.push(m + 1);
            stack.push(right);
            stack.push(next_axis);
        }
    }

    result
}

/*
fn sq_dist(ax: TNumber, ay: TNumber, bx: TNumber, by: TNumber) -> TNumber {
    (ax - bx).powi(2) + (ay - by).powi(2)
}
*/

fn sq_dist(ax: Num, ay: Num, bx: Num, by: Num) -> f32 {
    let dx = ax - bx;
    let dy = ay - by;
    dx * dx + dy * dy
}
