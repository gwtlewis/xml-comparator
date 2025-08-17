import http from 'k6/http';
import { check, sleep } from 'k6';
import { Counter, Rate, Trend } from 'k6/metrics';

// Custom metrics
const xmlPairsProcessed = new Counter('xml_pairs_processed');
const xmlPairsPerSecond = new Rate('xml_pairs_per_second');
const responseSizeBytes = new Trend('response_size_bytes');
const requestSizeBytes = new Trend('request_size_bytes');

// Test configuration
export const options = {
  scenarios: {
    smoke_test: {
      executor: 'shared-iterations',
      vus: 1,
      iterations: 1,
      maxDuration: '60s',
      tags: { test_type: 'smoke' },
      env: { TEST_SIZE: '100' }
    },
    
    nominal_load: {
      executor: 'shared-iterations', 
      vus: 1,
      iterations: 1,
      maxDuration: '1800s', // 30 minutes max for larger tests
      tags: { test_type: 'nominal' },
      env: { TEST_SIZE: __ENV.NOMINAL_SIZE || '100000' },
      startTime: '65s' // Start after smoke test
    },
    
    soak_test: {
      executor: 'shared-iterations',
      vus: 1, 
      iterations: 6, // 6 sequential batches
      maxDuration: '3600s', // 1 hour max
      tags: { test_type: 'soak' },
      env: { TEST_SIZE: '100000' },
      startTime: '970s' // Start after nominal test
    },
    
    stress_test: {
      executor: 'ramping-vus',
      startVUs: 1,
      stages: [
        { duration: '30s', target: 3 },
        { duration: '60s', target: 3 },
        { duration: '30s', target: 5 },
        { duration: '120s', target: 5 },
        { duration: '30s', target: 1 }
      ],
      tags: { test_type: 'stress' },
      env: { TEST_SIZE: '50000' },
      startTime: '4570s' // Start after soak test
    }
  },
  
  thresholds: {
    http_req_duration: ['p(95)<1800000'], // 95% under 30 minutes for large tests
    http_req_failed: ['rate<0.01'], // Less than 1% failure rate
    xml_pairs_per_second: ['rate>100'], // At least 100 pairs/sec
    'http_req_duration{test_type:smoke}': ['p(95)<60000'], // Smoke test under 1 min
    'http_req_duration{test_type:nominal}': ['p(95)<1800000'], // Nominal under 30 min for 600k
  }
};

export function setup() {
  console.log('=== Performance Test Setup ===');
  
  // Health check
  const healthRes = http.get(`${__ENV.BASE_URL || 'http://127.0.0.1:3000'}/xml-compare-api/health`);
  if (!check(healthRes, { 'health check passed': (r) => r.status === 200 })) {
    throw new Error('API health check failed');
  }
  
  console.log('API health check passed');
  return { baseUrl: __ENV.BASE_URL || 'http://127.0.0.1:3000' };
}

export default function(data) {
  const testSize = parseInt(__ENV.TEST_SIZE || '100');
  const testType = __ENV.SCENARIO || 'unknown';
  
  console.log(`Running ${testType} test with ${testSize} XML pairs`);
  
  // Generate payload on-the-fly to avoid large fixture files
  const payload = generateBatchPayload(testSize);
  const payloadStr = JSON.stringify(payload);
  
  requestSizeBytes.add(payloadStr.length);
  
  const params = {
    headers: {
      'Content-Type': 'application/json',
    },
    timeout: '900s', // 15 minute timeout
    tags: {
      test_type: testType,
      payload_size: testSize.toString()
    }
  };
  
  const startTime = Date.now();
  console.log(`Sending batch request at ${new Date(startTime).toISOString()}`);
  
  const response = http.post(
    `${data.baseUrl}/xml-compare-api/api/compare/xml/batch`,
    payloadStr,
    params
  );
  
  const endTime = Date.now();
  const duration = endTime - startTime;
  
  console.log(`Batch request completed in ${duration}ms`);
  
  // Validate response
  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'response has results': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.results && Array.isArray(body.results);
      } catch {
        return false;
      }
    },
    'all comparisons processed': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.total_comparisons === testSize;
      } catch {
        return false;
      }
    }
  });
  
  if (success) {
    const responseBody = JSON.parse(response.body);
    const pairsPerSecond = testSize / (duration / 1000);
    
    xmlPairsProcessed.add(testSize);
    xmlPairsPerSecond.add(pairsPerSecond);
    responseSizeBytes.add(response.body.length);
    
    console.log(`✓ Processed ${testSize} pairs in ${duration}ms (${pairsPerSecond.toFixed(2)} pairs/sec)`);
    console.log(`✓ Success rate: ${responseBody.successful_comparisons}/${responseBody.total_comparisons}`);
    console.log(`✓ Response size: ${(response.body.length / 1024 / 1024).toFixed(2)} MB`);
    
    // Log sample results for verification
    if (responseBody.results && responseBody.results.length > 0) {
      const sampleResult = responseBody.results[0];
      console.log(`✓ Sample result: matched=${sampleResult.matched}, diffs=${sampleResult.diffs.length}`);
    }
  } else {
    console.error(`✗ Test failed. Status: ${response.status}, Body length: ${response.body?.length || 0}`);
    if (response.body && response.body.length < 1000) {
      console.error(`Response body: ${response.body}`);
    }
  }
  
  // Add delay between iterations in soak test
  if (testType === 'soak') {
    console.log('Soak test: waiting 60s before next iteration...');
    sleep(60);
  }
}

export function teardown(data) {
  console.log('=== Performance Test Teardown ===');
  console.log('Test completed successfully');
}

function generateBatchPayload(count) {
  const comparisons = [];
  
  for (let i = 0; i < count; i++) {
    const depth = determineDepth(i, count);
    const xml1 = generateXML(depth, i, `doc${i}`);
    
    // 70% identical, 30% different
    const xml2 = Math.random() < 0.7 ? xml1 : generateXML(depth, i + 100000, `doc${i}_diff`);
    
    comparisons.push({
      xml1,
      xml2,
      ignore_paths: [],
      ignore_properties: []
    });
  }
  
  return { comparisons };
}

function determineDepth(index, total) {
  const percent = (index / total) * 100;
  if (percent < 10) return 5;    // First 10% get depth 5
  if (percent < 40) return 3;    // Next 30% get depth 3
  return 2;                      // Remaining 60% get depth 2
}

function generateXML(depth, seed, prefix) {
  if (depth === 0) {
    return `${prefix}_${seed}`;
  }
  
  const tag = `level${depth}`;
  const inner = generateXML(depth - 1, seed + 1, prefix);
  
  return `<${tag} id="${prefix}_${depth}" value="${seed}">${inner}</${tag}>`;
}
