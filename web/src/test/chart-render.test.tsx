import { describe, it, expect, beforeAll } from 'vitest';
import { render, act } from '@testing-library/react';
import React from 'react';
import { LineChart, Line, XAxis, YAxis, ResponsiveContainer } from 'recharts';

// Mock ResizeObserver for jsdom
beforeAll(() => {
  global.ResizeObserver = class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
});

const sampleData = [
  { date: '2026-05-14', total_weighted_tokens: 442 },
  { date: '2026-05-15', total_weighted_tokens: 934 },
];

function TestChartWithRC() {
  return (
    <div style={{ width: 500, height: 300 }}>
      <ResponsiveContainer width="100%" height={280}>
        <LineChart data={sampleData} margin={{ top: 5, right: 20, bottom: 25, left: 10 }}>
          <XAxis
            dataKey="date"
            stroke="rgba(255,255,255,0.15)"
            tick={{ fontSize: 11, fill: 'rgba(255,255,255,0.5)' }}
            tickFormatter={(v: string) => v.slice(5)}
          />
          <YAxis
            stroke="rgba(255,255,255,0.15)"
            tick={{ fontSize: 11, fill: 'rgba(255,255,255,0.5)' }}
          />
          <Line type="monotone" dataKey="total_weighted_tokens" stroke="#6366f1" />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}

describe('Chart with ResponsiveContainer', () => {
  it('renders XAxis tick labels via ResponsiveContainer', async () => {
    const { container } = render(<TestChartWithRC />);

    // Check what ResponsiveContainer renders
    const rcDiv = container.querySelector('.recharts-responsive-container');
    console.log('ResponsiveContainer div:', rcDiv ? 'found' : 'NOT FOUND');
    if (rcDiv) {
      const cs = window.getComputedStyle(rcDiv);
      console.log('RC div width:', rcDiv.clientWidth, 'computed:', cs.width);
      console.log('RC div height:', rcDiv.clientHeight, 'computed:', cs.height);
    }

    // Check for the inner div created by ResponsiveContainer
    const innerDivs = container.querySelectorAll('div');
    innerDivs.forEach((div, i) => {
      console.log(`div[${i}]: style="${div.getAttribute('style')}" w=${div.clientWidth} h=${div.clientHeight}`);
    });

    // Check for SVG
    const svg = container.querySelector('svg');
    console.log('\nSVG element:', svg ? 'found' : 'NOT FOUND');
    if (svg) {
      console.log('SVG viewBox:', svg.getAttribute('viewBox'));
      console.log('SVG width:', svg.getAttribute('width'), 'height:', svg.getAttribute('height'));
    } else {
      console.log('NO SVG RENDERED - ResponsiveContainer may have width=0');
      // Print the full innerHTML of the RC container
      if (rcDiv) {
        console.log('RC innerHTML:', rcDiv.innerHTML.slice(0, 500));
      }
    }

    // Check for tick text elements
    const ticks = container.querySelectorAll('.recharts-cartesian-axis-tick-value');
    console.log(`\nTick value elements: ${ticks.length}`);

    expect(ticks.length).toBeGreaterThan(0);
  });
});
