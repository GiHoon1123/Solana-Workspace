'use client';

import { useEffect, useState, useCallback } from 'react';

export type AlertType = 'success' | 'error' | 'info';

interface AlertToastProps {
  message: string;
  type?: AlertType;
  isOpen: boolean;
  onClose: () => void;
}

export default function AlertToast({ message, type = 'info', isOpen, onClose }: AlertToastProps) {
  const handleClose = useCallback(() => {
    onClose();
  }, [onClose]);

  useEffect(() => {
    if (!isOpen) return;
    
    // ESC 키로 닫기
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        handleClose();
      }
    };
    document.addEventListener('keydown', handleEscape);
    
    return () => {
      document.removeEventListener('keydown', handleEscape);
    };
  }, [isOpen, handleClose]);

  if (!isOpen) return null;

  const iconColor = {
    success: 'text-green-400',
    error: 'text-red-400',
    info: 'text-blue-400',
  }[type];

  const borderColor = {
    success: 'border-green-500',
    error: 'border-red-500',
    info: 'border-blue-500',
  }[type];

  const icon = {
    success: '✓',
    error: '✕',
    info: 'ℹ',
  }[type];

  return (
    <div className="w-full">
      <div
        className={`bg-gray-800 border-l-4 ${borderColor} rounded-lg shadow-2xl w-full overflow-hidden`}
      >
        <div className="p-4">
          <div className="flex items-center gap-3">
            <div className={`flex-shrink-0 w-8 h-8 rounded-full bg-gray-700 flex items-center justify-center text-lg font-bold ${iconColor}`}>
              {icon}
            </div>
            <div className="flex-1">
              <p className="text-white text-sm font-medium">{message}</p>
            </div>
            <button
              onClick={handleClose}
              className="flex-shrink-0 px-4 py-1.5 bg-gray-700 hover:bg-gray-600 text-white rounded transition-colors font-medium text-xs"
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
    <div className="fixed top-20 left-1/2 transform -translate-x-1/2 z-50 flex flex-col gap-2 pointer-events-none w-full max-w-md px-4">
      {alerts.map((alert, index) => (
        <div 
          key={alert.id} 
          className="pointer-events-auto"
          style={{ marginTop: `${index * 80}px` }}
        >
          <AlertToast
            message={alert.message}
            type={alert.type}
            isOpen={true}
            onClose={() => removeAlert(alert.id)}
          />
        </div>
      ))}
    </div>
  );

  return { showAlert, AlertContainer };
}

