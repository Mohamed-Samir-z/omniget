import { describe, expect, it } from 'vitest';
import { timeSeconds, validTimeRange, splitTime, joinTime } from './clip-time';
describe('clip time fields', () => {
  it('accepts seconds, M:S and H:M:S without needing to type a colon', () => {
    expect(timeSeconds(joinTime(['', '1', '45']))).toBe(105);
    expect(timeSeconds(joinTime(['1', '2', '3.5']))).toBe(3723.5);
    expect(splitTime('0:30')).toEqual(['', '0', '30']);
    expect(joinTime(['', '', ''])).toBe('');
    expect(timeSeconds('75')).toBe(75);
  });
  it('rejects malformed values and end-before-start', () => {
    for (const bad of ['1:60', '0:99:00', 'abc', '1..2', '-2', '1:2:3:4', 'Infinity', '0:0:0.1234']) expect(timeSeconds(bad)).toBeNull();
    expect(validTimeRange('0:30','1:45')).toBe(true);
    expect(validTimeRange('','1:00')).toBe(true);
    expect(validTimeRange('1:00','')).toBe(true);
    expect(validTimeRange('1:00','0:30')).toBe(false);
    expect(validTimeRange('1:00','1:00')).toBe(false);
    expect(validTimeRange('bad','')).toBe(false);
  });
});
