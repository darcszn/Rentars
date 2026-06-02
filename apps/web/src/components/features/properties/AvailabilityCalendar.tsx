'use client';

import { useCallback, useEffect, useState } from 'react';
import { Calendar, Trash2, Plus, X } from 'lucide-react';

interface AvailabilityRange {
  id: string;
  start_date: string;
  end_date: string;
  reason?: string;
}

interface AvailabilityCalendarProps {
  propertyId: string;
  onClose?: () => void;
}

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000';

/**
 * AvailabilityCalendar lets hosts block date ranges on their properties.
 * Blocked ranges are highlighted and prevent tenant booking.
 */
export default function AvailabilityCalendar({ propertyId, onClose }: AvailabilityCalendarProps) {
  const [ranges, setRanges] = useState<AvailabilityRange[]>([]);
  const [startDate, setStartDate] = useState('');
  const [endDate, setEndDate] = useState('');
  const [reason, setReason] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const today = new Date().toISOString().split('T')[0];

  const fetchRanges = useCallback(async () => {
    const res = await fetch(`${API_URL}/api/v1/properties/${propertyId}/availability`);
    if (res.ok) setRanges(await res.json());
  }, [propertyId]);

  useEffect(() => { fetchRanges(); }, [fetchRanges]);

  // Collect all blocked dates for calendar highlighting
  const blockedDates = new Set<string>();
  for (const range of ranges) {
    const start = new Date(range.start_date);
    const end = new Date(range.end_date);
    for (const d = new Date(start); d < end; d.setDate(d.getDate() + 1)) {
      blockedDates.add(d.toISOString().split('T')[0]);
    }
  }

  const handleAdd = async () => {
    setError('');
    if (!startDate || !endDate) { setError('Both dates are required'); return; }
    if (startDate >= endDate) { setError('Start date must be before end date'); return; }

    setLoading(true);
    const token = typeof window !== 'undefined' ? localStorage.getItem('token') : null;
    const res = await fetch(`${API_URL}/api/v1/properties/${propertyId}/availability`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` },
      body: JSON.stringify({ start_date: startDate, end_date: endDate, reason }),
    });
    setLoading(false);

    if (!res.ok) {
      const body = await res.json();
      setError(body.error || 'Failed to block dates');
      return;
    }

    setStartDate(''); setEndDate(''); setReason('');
    fetchRanges();
  };

  const handleDelete = async (rangeId: string) => {
    const token = typeof window !== 'undefined' ? localStorage.getItem('token') : null;
    await fetch(`${API_URL}/api/v1/properties/${propertyId}/availability/${rangeId}`, {
      method: 'DELETE',
      headers: { Authorization: `Bearer ${token}` },
    });
    fetchRanges();
  };

  // Render a mini calendar for the current month
  const now = new Date();
  const year = now.getFullYear();
  const month = now.getMonth();
  const daysInMonth = new Date(year, month + 1, 0).getDate();
  const firstDay = new Date(year, month, 1).getDay();
  const monthLabel = now.toLocaleString('default', { month: 'long', year: 'numeric' });

  return (
    <div className="bg-white rounded-lg shadow border border-gray-200 p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold flex items-center gap-2">
          <Calendar size={20} aria-hidden="true" />
          Availability Calendar
        </h3>
        {onClose && (
          <button onClick={onClose} aria-label="Close calendar" className="text-gray-400 hover:text-gray-600">
            <X size={20} aria-hidden="true" />
          </button>
        )}
      </div>

      {/* Mini calendar */}
      <div aria-label={`Calendar for ${monthLabel}`}>
        <p className="font-medium text-sm text-gray-700 mb-2">{monthLabel}</p>
        <div className="grid grid-cols-7 gap-1 text-xs text-center">
          {['Su', 'Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa'].map((d) => (
            <div key={d} className="font-semibold text-gray-500 py-1">{d}</div>
          ))}
          {Array.from({ length: firstDay }).map((_, i) => <div key={`e-${i}`} />)}
          {Array.from({ length: daysInMonth }, (_, i) => {
            const dateStr = `${year}-${String(month + 1).padStart(2, '0')}-${String(i + 1).padStart(2, '0')}`;
            const isBlocked = blockedDates.has(dateStr);
            const isPast = dateStr < today;
            return (
              <div
                key={dateStr}
                title={isBlocked ? 'Blocked' : ''}
                className={`py-1 rounded text-xs ${
                  isBlocked ? 'bg-red-200 text-red-700 font-semibold' :
                  isPast ? 'text-gray-300' : 'bg-gray-50 text-gray-700'
                }`}
              >
                {i + 1}
              </div>
            );
          })}
        </div>
        <div className="flex items-center gap-2 mt-2 text-xs text-gray-500">
          <span className="w-3 h-3 rounded bg-red-200 inline-block" aria-hidden="true" />
          Blocked
        </div>
      </div>

      {/* Add range form */}
      <div className="space-y-3">
        <p className="font-medium text-sm text-gray-800">Block a date range</p>
        <div className="grid grid-cols-2 gap-3">
          <div>
            <label className="text-xs text-gray-600 mb-1 block" htmlFor="avail-start">Start date</label>
            <input
              id="avail-start"
              type="date"
              min={today}
              value={startDate}
              onChange={(e) => setStartDate(e.target.value)}
              className="w-full border border-gray-300 rounded px-2 py-1.5 text-sm"
            />
          </div>
          <div>
            <label className="text-xs text-gray-600 mb-1 block" htmlFor="avail-end">End date</label>
            <input
              id="avail-end"
              type="date"
              min={startDate || today}
              value={endDate}
              onChange={(e) => setEndDate(e.target.value)}
              className="w-full border border-gray-300 rounded px-2 py-1.5 text-sm"
            />
          </div>
        </div>
        <div>
          <label className="text-xs text-gray-600 mb-1 block" htmlFor="avail-reason">Reason (optional)</label>
          <input
            id="avail-reason"
            type="text"
            placeholder="e.g. maintenance, personal use"
            value={reason}
            onChange={(e) => setReason(e.target.value)}
            className="w-full border border-gray-300 rounded px-2 py-1.5 text-sm"
          />
        </div>
        {error && <p className="text-red-600 text-xs" role="alert">{error}</p>}
        <button
          type="button"
          onClick={handleAdd}
          disabled={loading}
          className="flex items-center gap-1.5 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white text-sm font-medium px-3 py-2 rounded transition"
        >
          <Plus size={16} aria-hidden="true" />
          {loading ? 'Blocking...' : 'Block dates'}
        </button>
      </div>

      {/* Existing ranges */}
      {ranges.length > 0 && (
        <div className="space-y-2">
          <p className="font-medium text-sm text-gray-800">Blocked ranges</p>
          <ul className="space-y-1" aria-label="Blocked date ranges">
            {ranges.map((r) => (
              <li key={r.id} className="flex items-center justify-between bg-red-50 border border-red-100 rounded px-3 py-2 text-sm">
                <span>
                  <span className="font-medium">{r.start_date}</span>
                  {' → '}
                  <span className="font-medium">{r.end_date}</span>
                  {r.reason && <span className="text-gray-500 ml-2">({r.reason})</span>}
                </span>
                <button
                  type="button"
                  onClick={() => handleDelete(r.id)}
                  className="text-red-500 hover:text-red-700 ml-2"
                  aria-label={`Remove block from ${r.start_date} to ${r.end_date}`}
                >
                  <Trash2 size={14} aria-hidden="true" />
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
