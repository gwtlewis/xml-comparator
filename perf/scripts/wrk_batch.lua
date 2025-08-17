-- WRK2 Lua script for batch XML comparison performance testing
-- Usage: wrk -t4 -c4 -d300s -s wrk_batch.lua --latency http://127.0.0.1:3000/xml-compare-api/api/compare/xml/batch

-- Configuration
local test_size = 100000  -- Number of XML pairs per batch
local payload_cache = nil
local request_count = 0

-- Initialize the payload once
function setup(thread)
   math.randomseed(42)  -- Deterministic seed for reproducible results
   
   print("Generating batch payload with " .. test_size .. " XML pairs...")
   payload_cache = generate_batch_payload(test_size)
   print("Payload generated, size: " .. string.len(payload_cache) .. " bytes")
end

-- Called for each request
function request()
   request_count = request_count + 1
   
   -- Add request headers
   wrk.headers["Content-Type"] = "application/json"
   wrk.headers["Accept"] = "application/json"
   
   -- Log every 10th request
   if request_count % 10 == 0 then
      print("Sending request #" .. request_count)
   end
   
   return wrk.format("POST", nil, nil, payload_cache)
end

-- Called when response is received
function response(status, headers, body)
   if status ~= 200 then
      print("ERROR: HTTP " .. status)
      if string.len(body) < 500 then
         print("Response body: " .. body)
      end
      return
   end
   
   -- Parse and validate response
   local success, result = pcall(parse_json_response, body)
   if success and result then
      local pairs_processed = result.total_comparisons or 0
      local successful = result.successful_comparisons or 0
      
      if pairs_processed == test_size then
         print("✓ Processed " .. pairs_processed .. " pairs, " .. successful .. " successful")
      else
         print("⚠ Expected " .. test_size .. " pairs, got " .. pairs_processed)
      end
   else
      print("✗ Failed to parse response JSON")
   end
end

-- Generate batch payload JSON
function generate_batch_payload(count)
   local comparisons = {}
   
   for i = 1, count do
      local depth = determine_depth(i, count)
      local seed = i * 7 + 123  -- Simple deterministic seed
      
      local xml1 = generate_xml(depth, seed, "doc" .. i)
      local xml2
      
      -- 70% identical, 30% different
      if math.random() < 0.7 then
         xml2 = xml1
      else
         xml2 = generate_xml(depth, seed + 100000, "doc" .. i .. "_diff")
      end
      
      table.insert(comparisons, {
         xml1 = xml1,
         xml2 = xml2,
         ignore_paths = {},
         ignore_properties = {}
      })
   end
   
   local payload = {
      comparisons = comparisons
   }
   
   return json_encode(payload)
end

-- Determine XML depth based on index
function determine_depth(index, total)
   local percent = (index / total) * 100
   if percent <= 10 then
      return 5  -- First 10% get depth 5
   elseif percent <= 40 then
      return 3  -- Next 30% get depth 3
   else
      return 2  -- Remaining 60% get depth 2
   end
end

-- Generate XML recursively
function generate_xml(depth, seed, prefix)
   if depth == 0 then
      return prefix .. "_" .. seed
   end
   
   local tag = "level" .. depth
   local inner = generate_xml(depth - 1, seed + 1, prefix)
   
   return "<" .. tag .. ' id="' .. prefix .. "_" .. depth .. '" value="' .. seed .. '">' .. inner .. "</" .. tag .. ">"
end

-- Simple JSON encoder (minimal implementation for our needs)
function json_encode(obj)
   if type(obj) == "table" then
      if #obj > 0 then
         -- Array
         local parts = {}
         for i, v in ipairs(obj) do
            table.insert(parts, json_encode(v))
         end
         return "[" .. table.concat(parts, ",") .. "]"
      else
         -- Object
         local parts = {}
         for k, v in pairs(obj) do
            table.insert(parts, '"' .. k .. '":' .. json_encode(v))
         end
         return "{" .. table.concat(parts, ",") .. "}"
      end
   elseif type(obj) == "string" then
      return '"' .. obj:gsub('"', '\\"') .. '"'
   elseif type(obj) == "number" then
      return tostring(obj)
   elseif type(obj) == "boolean" then
      return obj and "true" or "false"
   else
      return "null"
   end
end

-- Simple JSON parser for response validation
function parse_json_response(body)
   -- Very basic JSON parsing - extract key fields
   local total_comparisons = body:match('"total_comparisons":(%d+)')
   local successful_comparisons = body:match('"successful_comparisons":(%d+)')
   
   if total_comparisons and successful_comparisons then
      return {
         total_comparisons = tonumber(total_comparisons),
         successful_comparisons = tonumber(successful_comparisons)
      }
   end
   
   return nil
end

-- Called at the end to print summary
function done(summary, latency, requests)
   print("\n=== WRK Performance Test Summary ===")
   print("Requests completed: " .. summary.requests)
   print("Duration: " .. summary.duration / 1000000 .. "s")
   print("Total data transferred: " .. summary.bytes .. " bytes")
   print("Average latency: " .. latency.mean / 1000 .. "ms")
   print("99th percentile latency: " .. latency.p99 / 1000 .. "ms")
   print("Requests per second: " .. (summary.requests / (summary.duration / 1000000)))
   
   if summary.requests > 0 then
      local total_pairs = summary.requests * test_size
      local pairs_per_second = total_pairs / (summary.duration / 1000000)
      print("Total XML pairs processed: " .. total_pairs)
      print("XML pairs per second: " .. math.floor(pairs_per_second))
   end
   
   print("Errors: " .. summary.errors.connect .. " connect, " .. 
         summary.errors.read .. " read, " .. 
         summary.errors.write .. " write, " .. 
         summary.errors.status .. " status, " .. 
         summary.errors.timeout .. " timeout")
end
