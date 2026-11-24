-- ClassicAPI's C_CurveUtil.CreateCurve (curve/Curve.cpp, as the 3.80.2 client has it): a scalar
-- curve of at most 256 points kept sorted by x, a new point going before the points of the same
-- x. Evaluate: no points 0, one point its value; below the first point or past the last the end
-- point's value (no extrapolation). Linear interpolates, Step takes the last point at or before
-- x, Cosine eases by (1 - cos(t pi)) / 2, Cubic is a uniform Catmull-Rom over the points from the
-- second to the one before last (the ends only steer it, flat outside them), Cosine with fewer
-- than four points. Colour curves, secret values and game curves are not served here.
Enum.LuaCurveType = Enum.LuaCurveType or { Linear = 0, Step = 1, Cosine = 2, Cubic = 3 }

local LINEAR, STEP, COSINE, CUBIC = 0, 1, 2, 3
local MAX_POINTS = 256

local Curve = {}
local CurveMeta = { __index = Curve }

local function number(value, method, slot)
  local n = tonumber(value)
  if not n then error("Usage: LuaCurveObject:" .. method .. "(): bad argument #" .. slot, 3) end
  return n
end

function Curve:GetType() return self.type end
function Curve:SetType(kind)
  local t = math.floor(number(kind, "SetType", 1))
  if t < LINEAR or t > CUBIC then error("LuaCurveObject:SetType(): invalid curve type", 2) end
  self.type = t
end
function Curve:HasSecretValues() return false end
function Curve:SetToDefaults() self.points = {}; self.type = LINEAR end
function Curve:ClearPoints() self.points = {} end
function Curve:GetPointCount() return table.getn(self.points) end
function Curve:AddPoint(x, y)
  x, y = number(x, "AddPoint", 1), number(y, "AddPoint", 2)
  local points = self.points
  if table.getn(points) >= MAX_POINTS then
    error("LuaCurveObject:AddPoint(): too many points", 2)
  end
  -- Before the first point of the same or a larger x.
  local at = table.getn(points) + 1
  for i, p in ipairs(points) do
    if p.x >= x then at = i; break end
  end
  table.insert(points, at, { x = x, y = y })
end
function Curve:GetPoint(index)
  local p = self.points[tonumber(index) or 0]
  if p then return { x = p.x, y = p.y } end
end
function Curve:GetPoints()
  local out = {}
  for i, p in ipairs(self.points) do out[i] = { x = p.x, y = p.y } end
  return out
end
function Curve:RemovePoint(index)
  index = tonumber(index)
  if not index or not self.points[index] then
    error("LuaCurveObject:RemovePoint(): no point at that index", 2)
  end
  table.remove(self.points, index)
end
function Curve:SetPoints(list)
  local points = {}
  for i, p in ipairs(list or {}) do
    points[i] = { x = number(p.x, "SetPoints", 1), y = number(p.y, "SetPoints", 1) }
  end
  if table.getn(points) > MAX_POINTS then error("LuaCurveObject:SetPoints(): too many points", 2) end
  table.sort(points, function(a, b) return a.x < b.x end)
  self.points = points
end
function Curve:Copy()
  local copy = setmetatable({ points = {}, type = self.type }, CurveMeta)
  for i, p in ipairs(self.points) do copy.points[i] = { x = p.x, y = p.y } end
  return copy
end

-- The first point in first..last past x (std::upper_bound).
local function upper(points, first, last, x)
  for i = first, last do
    if x < points[i].x then return i end
  end
  return last + 1
end
local function linear(a, b, x)
  local dx = b.x - a.x
  if dx <= 0 then return b.y end
  return (x - a.x) / dx * (b.y - a.y) + a.y
end
local function cosine(a, b, x)
  local dx = b.x - a.x
  if dx <= 0 then return b.y end
  return (1 - math.cos((x - a.x) / dx * math.pi)) * 0.5 * (b.y - a.y) + a.y
end
local function catmullRom(v0, p1, p2, v3, x)
  local dx = p2.x - p1.x
  if dx <= 0 then return p1.y end
  local t, v1, v2 = (x - p1.x) / dx, p1.y, p2.y
  return ((1.5 * v1 - 0.5 * v0 - 1.5 * v2 + 0.5 * v3) * t + (v0 - 2.5 * v1 + 2 * v2 - 0.5 * v3)) * t * t
    + (0.5 * v2 - 0.5 * v0) * t + v1
end

function Curve:Evaluate(x)
  x = number(x, "Evaluate", 1)
  local points = self.points
  local n = table.getn(points)
  if n == 0 then return 0 end
  if n == 1 then return points[1].y end
  if self.type == CUBIC and n >= 4 then
    local it = upper(points, 2, n - 1, x)
    if it == 2 then return points[2].y end
    if it == n then return points[n - 1].y end
    return catmullRom(points[it - 2].y, points[it - 1], points[it], points[it + 1].y, x)
  end
  local it = upper(points, 1, n, x)
  if it == 1 then return points[1].y end
  if it == n + 1 then return points[n].y end
  local a, b = points[it - 1], points[it]
  if self.type == LINEAR then return linear(a, b, x) end
  if self.type == STEP then return a.y end
  return cosine(a, b, x)
end

C_CurveUtil = C_CurveUtil or {}
function C_CurveUtil.CreateCurve()
  return setmetatable({ points = {}, type = LINEAR }, CurveMeta)
end
