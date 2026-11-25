-- ClassicAPI's number text (`localization/Numbers.cpp`): `BreakUpLargeNumbers`,
-- `AbbreviateNumbers`, `AbbreviateLargeNumbers` and `C_StringUtil.GetDefaultAbbreviationBreakpoints`,
-- with `Constants.MoneyFormattingConstants` (`currency/MoneyConstants.cpp`). The labels and the
-- separators are global strings, read at the call ("" when a global is not a string); the
-- `breakUpLargeNumbers` CVar at "0" drops the digit separators.

local function global(name)
  local value = getglobal(name)
  if type(value) == "string" then return value end
  return ""
end

local function asian(locale)
  return locale == "koKR" or locale == "zhCN" or locale == "zhTW"
end

local NUMBER = {
  { 1e13, "FOURTH_NUMBER_CAP_NO_SPACE", 1e12, 1 },
  { 1e12, "FOURTH_NUMBER_CAP_NO_SPACE", 1e11, 10 },
  { 1e10, "THIRD_NUMBER_CAP_NO_SPACE", 1e9, 1 },
  { 1e9, "THIRD_NUMBER_CAP_NO_SPACE", 1e8, 10 },
  { 1e7, "SECOND_NUMBER_CAP_NO_SPACE", 1e6, 1 },
  { 1e6, "SECOND_NUMBER_CAP_NO_SPACE", 1e5, 10 },
  { 1e4, "FIRST_NUMBER_CAP_NO_SPACE", 1e3, 1 },
  { 1e3, "FIRST_NUMBER_CAP_NO_SPACE", 1e2, 10 },
}
local NUMBER_ASIAN = {
  { 1e9, "SECOND_NUMBER_CAP_NO_SPACE", 1e8, 1 },
  { 1e8, "SECOND_NUMBER_CAP_NO_SPACE", 1e7, 10 },
  { 1e5, "FIRST_NUMBER_CAP_NO_SPACE", 1e4, 1 },
  { 1e4, "FIRST_NUMBER_CAP_NO_SPACE", 1e3, 10 },
}
local LARGE = {
  { 1e8, "SECOND_NUMBER_CAP", 1e6, 1 },
  { 1e5, "FIRST_NUMBER_CAP", 1e3, 1 },
}
local LARGE_ASIAN = {
  { 1e10, "SECOND_NUMBER_CAP", 1e8, 1 },
  { 1e8, "SECOND_NUMBER_CAP", 1e6, 100 },
  { 1e6, "FIRST_NUMBER_CAP", 1e4, 1 },
}
local SCALES = { { 1e12, "FOURTH_NUMBER" }, { 1e9, "THIRD_NUMBER" }, { 1e6, "SECOND_NUMBER" } }
local SCALES_ASIAN = { { 1e12, "FOURTH_NUMBER" }, { 1e8, "THIRD_NUMBER" }, { 1e4, "SECOND_NUMBER" } }

local function whole(w) return string.format("%.0f", w) end
local function luanumber(v) return string.format("%.14g", v) end

local function grouped(w)
  local digits = whole(w)
  if GetCVar("breakUpLargeNumbers") == "0" then return digits end
  local sep, out, n = global("LARGE_NUMBER_SEPERATOR"), "", string.len(digits)
  for i = 1, n do
    if i > 1 and math.mod(n - i + 1, 3) == 0 then out = out .. sep end
    out = out .. string.sub(digits, i, i)
  end
  return out
end

local function signed_whole(v)
  local w = math.floor(math.abs(v))
  return ((v < 0 and w ~= 0) and "-" or "") .. grouped(w)
end

local function break_up(v, natural)
  if natural then
    for _, s in ipairs(asian(GetLocale()) and SCALES_ASIAN or SCALES) do
      if v >= s[1] then
        local tenths = math.floor(v / (s[1] / 10))
        local w = math.floor(tenths / 10)
        local num, digit = whole(w), tenths - w * 10
        if tenths < 1000 and digit ~= 0 then
          num = num .. global("DECIMAL_SEPERATOR") .. whole(digit)
        end
        return num .. " " .. global(s[2])
      end
    end
    return signed_whole(v)
  end
  local a = math.abs(v)
  if a >= 1000 or a == math.floor(a) then return signed_whole(v) end
  local tenths = math.floor(a * 10 + 0.5)
  local w = math.floor(tenths / 10)
  return (v < 0 and "-" or "") .. whole(w) .. global("DECIMAL_SEPERATOR") .. whole(tenths - w * 10)
end

local function abbreviate(v, significand, fraction, label, round_whole)
  local q = math.floor(v / significand) / fraction
  if round_whole then q = math.floor(q + 0.5) end
  return luanumber(q) .. label
end

local function number_arg(value, usage)
  local v = tonumber(value)
  if not v then error("Usage: " .. usage, 3) end
  return v
end

local function field(entry, key, fn)
  local v = tonumber(entry[key])
  if not v then error(fn .. ": breakpointData entry needs a number '" .. key .. "'", 4) end
  return v
end

-- The first breakpoint at or under `v`, from the caller's data or the defaults; nil for none.
local function walk(v, options, defaults, round_whole, fn)
  local data = type(options) == "table" and type(options.breakpointData) == "table"
    and options.breakpointData
  if data then
    local i = 1
    while data[i] ~= nil do
      local entry = data[i]
      if type(entry) ~= "table" then
        error(fn .. ": breakpointData entry " .. i .. " is not a table", 3)
      end
      if v >= field(entry, "breakpoint", fn) then
        local significand = field(entry, "significandDivisor", fn)
        local fraction = field(entry, "fractionDivisor", fn)
        if type(entry.abbreviation) ~= "string" then
          error(fn .. ": breakpointData entry needs a string 'abbreviation'", 3)
        end
        local is_global = entry.abbreviationIsGlobal == nil or entry.abbreviationIsGlobal
        local label = is_global and global(entry.abbreviation) or entry.abbreviation
        return abbreviate(v, significand, fraction, label, round_whole)
      end
      i = i + 1
    end
    return nil
  end
  for _, b in ipairs(defaults) do
    if v >= b[1] then return abbreviate(v, b[3], b[4], global(b[2]), round_whole) end
  end
end

local function options_asian(options)
  if type(options) == "table" and type(options.locale) == "string" then
    return asian(options.locale)
  end
  return asian(GetLocale())
end

function BreakUpLargeNumbers(value, natural)
  return break_up(number_arg(value, "BreakUpLargeNumbers(largeNumber [, natural])"),
    natural and true or false)
end

function AbbreviateNumbers(value, options)
  local v = number_arg(value, "AbbreviateNumbers(number [, options])")
  local data = options_asian(options) and NUMBER_ASIAN or NUMBER
  return walk(v, options, data, false, "AbbreviateNumbers") or luanumber(v)
end

function AbbreviateLargeNumbers(value, options)
  local v = number_arg(value, "AbbreviateLargeNumbers(number [, options])")
  local is_asian = options_asian(options)
  return walk(v, options, is_asian and LARGE_ASIAN or LARGE, not is_asian,
    "AbbreviateLargeNumbers") or break_up(v, false)
end

C_StringUtil = C_StringUtil or {}
function C_StringUtil.GetDefaultAbbreviationBreakpoints(locale)
  local data = asian(type(locale) == "string" and locale or GetLocale()) and NUMBER_ASIAN or NUMBER
  local out = {}
  for i, b in ipairs(data) do
    out[i] = {
      breakpoint = b[1], abbreviation = b[2], significandDivisor = b[3],
      fractionDivisor = b[4], abbreviationIsGlobal = true,
    }
  end
  return out
end

Constants = Constants or {}
Constants.MoneyFormattingConstants = { GOLD_REWARD_THRESHOLD_TO_HIDE_COPPER = 10 }
