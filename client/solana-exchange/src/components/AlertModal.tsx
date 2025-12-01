'use client';

import { useEffect, useState } from 'react';

export type AlertType = 'success' | 'error' | 'info';

interface AlertModalProps {
  message: string;
  type?: AlertType;
  isOpen: boolean;
  onClose: () => void;
}

export default function AlertModal({ message, type = 'info', isOpen, onClose }: AlertModalProps) {
  useEffect(() => {
    if (isOpen) {
      // ESC 키로 닫기
      const handleEscape = (e: KeyboardEvent) => {
        if (e.key === 'Escape') {
          onClose();
        }
      };
      document.addEventListener('keydown', handleEscape);
      return () => document.removeEventListener('keydown', handleEscape);
    }
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const iconColor = {
    success: 'text-green-400',
    error: 'text-red-400',
    info: 'text-blue-400',
  }[type];

  const icon = {
    success: '✓',
    error: '✕',
    info: 'ℹ',
  }[type];

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-5"
      onClick={onClose}
    >
      <div
        className="bg-gray-800 border border-gray-700 rounded-lg shadow-2xl max-w-md w-full mx-4 overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="p-6">
          <div className="flex items-start gap-4 mb-6">
            <div className={`flex-shrink-0 w-12 h-12 rounded-full bg-gray-700 flex items-center justify-center text-2xl font-bold ${iconColor}`}>
              {icon}
            </div>
            <div className="flex-1 pt-1">
              <p className="text-white text-base font-medium leading-relaxed">{message}</p>
            </div>
          </div>
          <div className="flex justify-end">
            <button
              onClick={onClose}
              className="px-6 py-2.5 bg-gray-700 hover:bg-gray-600 text-white rounded transition-colors font-medium text-sm"
            >
              확인
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

// Alert Manager Hook
interface AlertItem {
  id: string;
  message: string;
  type: AlertType;
}

export function useAlert() {
  const [alerts, setAlerts] = useState<AlertItem[]>([]);

  const showAlert = (message: string, type: AlertType = 'info') => {
    const id = Math.random().toString(36).substr(2, 9);
    setAlerts((prev) => [...prev, { id, message, type }]);
  };

  const removeAlert = (id: string) => {
    setAlerts((prev) => prev.filter((alert) => alert.id !== id));
  };

  const AlertContainer = () => (
    <>
      {alerts.map((alert) => (
        <AlertModal
          key={alert.id}
          message={alert.message}
          type={alert.type}
          isOpen={true}
          onClose={() => removeAlert(alert.id)}
        />
      ))}
    </>
  );

  return { showAlert, AlertContainer };
}

