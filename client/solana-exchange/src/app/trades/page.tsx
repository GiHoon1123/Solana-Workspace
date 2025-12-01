'use client';

import { useEffect } from 'react';
import { useRouter } from 'next/navigation';
import { apiClient } from '@/lib/api';
import OrderManagement from '@/components/OrderManagement';

export default function TradesPage() {
  const router = useRouter();

  useEffect(() => {
    if (!apiClient.isAuthenticated()) {
      router.push('/');
    }
  }, [router]);

  if (!apiClient.isAuthenticated()) {
    return null;
  }

  return (
    <main className="min-h-screen bg-gray-900">
      <div className="max-w-[1920px] mx-auto px-6 py-8">
        <h1 className="text-3xl font-bold text-white mb-8">거래내역</h1>
        <div className="h-[calc(100vh-12rem)] min-h-[600px]">
          <OrderManagement />
        </div>
      </div>
    </main>
  );
}

