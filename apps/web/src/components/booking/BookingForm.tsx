'use client';

import { useEffect, useState } from 'react';
import { Calendar, Users } from 'lucide-react';

interface BookingFormProps {
  propertyId: string;
  pricePerNight: number;
  onSubmit: (data: { checkIn: Date; checkOut: Date; guestCount: number }) => void;
  isLoading?: boolean;
}

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000';

export default function BookingForm({ propertyId, pricePerNight, onSubmit, isLoading = false }: BookingFormProps) {
  const [checkIn, setCheckIn] = useState('');
  const [checkOut, setCheckOut] = useState('');
  const [guestCount, setGuestCount] = useState(1);
  const [blockedDates, setBlockedDates] = useState<Set<string>>(new Set());
  const [dateError, setDateError] = useState('');

  const today = new Date().toISOString().split('T')[0];

  useEffect(() => {
    fetch(`${API_URL}/api/v1/properties/${propertyId}/availability`)
      .then((r) => r.ok ? r.json() : [])
      .then((ranges: Array<{ start_date: string; end_date: string }>) => {
        const dates = new Set<string>();
        for (const range of ranges) {
          const start = new Date(range.start_date);
          const end = new Date(range.end_date);
          for (const d = new Date(start); d < end; d.setDate(d.getDate() + 1)) {
            dates.add(d.toISOString().split('T')[0]);
          }
        }
        setBlockedDates(dates);
      })
      .catch(() => {});
  }, [propertyId]);

  const isRangeBlocked = (start: string, end: string): boolean => {
    if (!start || !end) return false;
    const s = new Date(start);
    const e = new Date(end);
    for (const d = new Date(s); d < e; d.setDate(d.getDate() + 1)) {
      if (blockedDates.has(d.toISOString().split('T')[0])) return true;
    }
    return false;
  };

  const nights = checkIn && checkOut
    ? Math.ceil((new Date(checkOut).getTime() - new Date(checkIn).getTime()) / 86400000)
    : 0;

  const totalPrice = Math.max(0, nights) * pricePerNight;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setDateError('');
    if (!checkIn || !checkOut || nights <= 0) {
      setDateError('Please select valid dates');
      return;
    }
    if (isRangeBlocked(checkIn, checkOut)) {
      setDateError('Selected dates include unavailable periods. Please choose different dates.');
      return;
    }
    onSubmit({ checkIn: new Date(checkIn), checkOut: new Date(checkOut), guestCount });
  };

  return (
    <form onSubmit={handleSubmit} className="bg-white rounded-lg shadow-md p-6 space-y-4">
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1" htmlFor="check-in">
            <Calendar className="inline mr-2" size={16} aria-hidden="true" />
            Check-in
          </label>
          <input
            id="check-in"
            type="date"
            min={today}
            value={checkIn}
            onChange={(e) => { setCheckIn(e.target.value); setDateError(''); }}
            className="w-full border border-gray-300 rounded-lg px-3 py-2"
            required
            aria-describedby={dateError ? 'date-error' : undefined}
          />
        </div>
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1" htmlFor="check-out">
            <Calendar className="inline mr-2" size={16} aria-hidden="true" />
            Check-out
          </label>
          <input
            id="check-out"
            type="date"
            min={checkIn || today}
            value={checkOut}
            onChange={(e) => { setCheckOut(e.target.value); setDateError(''); }}
            className="w-full border border-gray-300 rounded-lg px-3 py-2"
            required
          />
        </div>
      </div>

      {dateError && (
        <p id="date-error" className="text-red-600 text-sm" role="alert">{dateError}</p>
      )}

      <div>
        <label className="block text-sm font-medium text-gray-700 mb-1" htmlFor="guests">
          <Users className="inline mr-2" size={16} aria-hidden="true" />
          Guests
        </label>
        <input
          id="guests"
          type="number"
          min="1"
          value={guestCount}
          onChange={(e) => setGuestCount(parseInt(e.target.value))}
          className="w-full border border-gray-300 rounded-lg px-3 py-2"
        />
      </div>

      <div className="bg-gray-50 p-4 rounded-lg space-y-2">
        <div className="flex justify-between text-sm">
          <span className="text-gray-600">{pricePerNight} USDC × {nights} nights</span>
          <span className="font-medium">{totalPrice} USDC</span>
        </div>
        <div className="border-t pt-2 flex justify-between font-semibold">
          <span>Total</span>
          <span className="text-blue-600">{totalPrice} USDC</span>
        </div>
      </div>

      <button
        type="submit"
        disabled={isLoading || nights <= 0}
        className="w-full bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 text-white font-medium py-2 px-4 rounded-lg transition"
      >
        {isLoading ? 'Processing...' : 'Book Now'}
      </button>
    </form>
  );
}
