'use client';

import AvailabilityCalendar from '@/components/features/properties/AvailabilityCalendar';

interface CalendarModalProps {
  isOpen: boolean;
  onClose: () => void;
  propertyId: string;
}

export default function CalendarModal({ isOpen, onClose, propertyId }: CalendarModalProps) {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4" role="dialog" aria-modal="true" aria-label="Manage Availability">
      <div className="bg-white rounded-xl shadow-xl max-w-lg w-full max-h-[90vh] overflow-y-auto">
        <div className="p-6">
          <AvailabilityCalendar propertyId={propertyId} onClose={onClose} />
        </div>
      </div>
    </div>
  );
}
