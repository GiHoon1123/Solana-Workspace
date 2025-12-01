'use client';

import { useState, useEffect, useRef } from 'react';
import { apiClient, Balance, CreateOrderRequest } from '@/lib/api';
import { useAlert } from './AlertModal';

type OrderType = 'buy' | 'sell';
type OrderSide = 'limit' | 'market';

export default function OrderPanel() {
  const [orderType, setOrderType] = useState<OrderType>('buy');
  const [orderSide, setOrderSide] = useState<OrderSide>('limit');
  const [price, setPrice] = useState('');
  const [amount, setAmount] = useState('');
  const [quoteAmount, setQuoteAmount] = useState(''); // 시장가 매수용 금액
  const [total, setTotal] = useState('');
  const [solPrice, setSolPrice] = useState<number | null>(null); // 실시간 SOL 가격
  const [balances, setBalances] = useState<Balance[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const { showAlert, AlertContainer } = useAlert();

  const wsTickerRef = useRef<WebSocket | null>(null);

  // 실시간 SOL 가격 가져오기 (WebSocket)
  useEffect(() => {
    // 초기 가격 로드 (REST API)
    const fetchInitialSolPrice = async () => {
      try {
        const response = await fetch('https://api.binance.com/api/v3/ticker/price?symbol=SOLUSDT');
        const data = await response.json();
        const price = parseFloat(data.price) || null;
        setSolPrice(price);
      } catch (error) {
        console.error('초기 SOL 가격 가져오기 실패:', error);
      }
    };
    fetchInitialSolPrice();

    // WebSocket 연결
    wsTickerRef.current = new WebSocket('wss://stream.binance.com:9443/ws/solusdt@ticker');

    wsTickerRef.current.onopen = () => {
      console.log('OrderPanel: SOL 가격 WebSocket 연결됨');
    };

    wsTickerRef.current.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        const price = parseFloat(data.c) || null; // 현재가 (last price)
        if (price && price > 0) {
          setSolPrice(price);
        }
      } catch (error) {
        console.error('OrderPanel: SOL 가격 WebSocket 데이터 파싱 실패:', error);
      }
    };

    wsTickerRef.current.onerror = (error) => {
      console.warn('OrderPanel: SOL 가격 WebSocket 연결 오류');
    };

    wsTickerRef.current.onclose = () => {
      console.warn('OrderPanel: SOL 가격 WebSocket 연결 종료');
    };

    return () => {
      if (wsTickerRef.current) {
        wsTickerRef.current.close();
      }
    };
  }, []);

  // 잔액 가져오기
  useEffect(() => {
    const fetchBalances = async () => {
      if (!apiClient.isAuthenticated()) {
        setBalances([]);
        return;
      }

      try {
        const response = await apiClient.getBalances();
        if (response.balances && response.balances.length > 0) {
          setBalances(response.balances);
        }
      } catch (error) {
        console.error('잔액 가져오기 실패:', error);
      }
    };

    fetchBalances();
    const interval = setInterval(fetchBalances, 5000);
    return () => clearInterval(interval);
  }, []);

  const getBalance = (mint: string): number => {
    const balance = balances.find(b => b.mint_address === mint);
    if (!balance) return 0;
    return parseFloat(balance.available) || 0;
  };

  const handlePriceChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setPrice(value);
    if (value && amount) {
      const priceNum = parseFloat(value);
      const amountNum = parseFloat(amount);
      if (!isNaN(priceNum) && !isNaN(amountNum) && priceNum > 0 && amountNum > 0) {
        setTotal((priceNum * amountNum).toFixed(2));
      } else {
        setTotal('');
      }
    } else {
      setTotal('');
    }
  };

  const handleAmountChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setAmount(value);
    if (value && price) {
      const priceNum = parseFloat(price);
      const amountNum = parseFloat(value);
      if (!isNaN(priceNum) && !isNaN(amountNum) && priceNum > 0 && amountNum > 0) {
        setTotal((priceNum * amountNum).toFixed(2));
      } else {
        setTotal('');
      }
    } else {
      setTotal('');
    }
  };

  const handleQuoteAmountChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setQuoteAmount(value);
    // 시장가 매수는 총액이 곧 quoteAmount
    setTotal(value);
  };

  const handleSetCurrentPrice = () => {
    if (solPrice) {
      setPrice(solPrice.toFixed(2));
      if (amount) {
        setTotal((solPrice * parseFloat(amount)).toFixed(2));
      }
    }
  };

  const setPercentage = (percent: number) => {
    setError(null);
    setSuccess(null);

    if (orderType === 'buy') {
      const usdtBalance = getBalance('USDT');
      if (usdtBalance <= 0) {
        setError('USDT 잔액이 없습니다.');
        return;
      }
      const calculatedAmount = ((usdtBalance * percent) / 100).toFixed(2);
      if (orderSide === 'limit') {
        // 지정가 매수: USDT 잔액으로 살 수 있는 SOL 수량 계산
        if (price && parseFloat(price) > 0) {
          setAmount((parseFloat(calculatedAmount) / parseFloat(price)).toFixed(4));
          setTotal(calculatedAmount);
        } else {
          setError('가격을 먼저 입력해주세요.');
        }
      } else {
        // 시장가 매수: USDT 잔액의 일정 비율을 금액으로 설정
        setQuoteAmount(calculatedAmount);
        setTotal(calculatedAmount);
      }
    } else {
      // 매도
      const solBalance = getBalance('SOL');
      if (solBalance <= 0) {
        setError('SOL 잔액이 없습니다.');
        return;
      }
      const calculatedAmount = ((solBalance * percent) / 100).toFixed(4);
      setAmount(calculatedAmount);
      if (orderSide === 'limit' && price) {
        setTotal((parseFloat(price) * parseFloat(calculatedAmount)).toFixed(2));
      } else if (orderSide === 'market' && solPrice) {
        setTotal((solPrice * parseFloat(calculatedAmount)).toFixed(2));
      } else {
        setTotal('');
      }
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSuccess(null);
    setLoading(true);

    if (!apiClient.isAuthenticated()) {
      setError('로그인이 필요합니다.');
      setLoading(false);
      return;
    }

    try {
      // 유효성 검증
      const currentBalances = (await apiClient.getBalances()).balances;
      console.log('주문 전 잔액 조회:', currentBalances);

      if (orderSide === 'limit') {
        if (!price || parseFloat(price) <= 0) {
          throw new Error('가격을 입력해주세요.');
        }
        if (!amount || parseFloat(amount) <= 0) {
          throw new Error('수량을 입력해주세요.');
        }

        // 잔액 체크
        if (orderType === 'buy') {
          const requiredUsdt = parseFloat(price) * parseFloat(amount);
          const usdtBalance = currentBalances.find(b => b.mint_address === 'USDT');
          console.log('USDT 잔액 확인:', usdtBalance);
          const availableUsdt = usdtBalance ? parseFloat(usdtBalance.available) : 0;
          console.log(`필요: ${requiredUsdt}, 보유: ${availableUsdt}`);
          if (requiredUsdt > availableUsdt) {
            throw new Error(`USDT 잔액이 부족합니다. (필요: ${requiredUsdt.toLocaleString()} USDT, 보유: ${availableUsdt.toLocaleString()} USDT)`);
          }
        } else {
          // 매도
          const requiredSol = parseFloat(amount);
          const solBalance = currentBalances.find(b => b.mint_address === 'SOL');
          const availableSol = solBalance ? parseFloat(solBalance.available) : 0;
          if (requiredSol > availableSol) {
            throw new Error(`SOL 잔액이 부족합니다. (필요: ${requiredSol.toFixed(4)} SOL, 보유: ${availableSol.toFixed(4)} SOL)`);
          }
        }
      } else {
        // 시장가
        if (orderType === 'buy') {
          if (!quoteAmount || parseFloat(quoteAmount) <= 0) {
            throw new Error('금액을 입력해주세요.');
          }
          
          // 잔액 체크
          const requiredUsdt = parseFloat(quoteAmount);
          const usdtBalance = currentBalances.find(b => b.mint_address === 'USDT');
          console.log('시장가 매수 - USDT 잔액 확인:', usdtBalance);
          const availableUsdt = usdtBalance ? parseFloat(usdtBalance.available) : 0;
          console.log(`필요: ${requiredUsdt}, 보유: ${availableUsdt}`);
          if (requiredUsdt > availableUsdt) {
            throw new Error(`USDT 잔액이 부족합니다. (필요: ${requiredUsdt.toLocaleString()} USDT, 보유: ${availableUsdt.toLocaleString()} USDT)`);
          }
        } else {
          // 시장가 매도
          if (!amount || parseFloat(amount) <= 0) {
            throw new Error('수량을 입력해주세요.');
          }
          
          // 잔액 체크
          const requiredSol = parseFloat(amount);
          const solBalance = currentBalances.find(b => b.mint_address === 'SOL');
          const availableSol = solBalance ? parseFloat(solBalance.available) : 0;
          if (requiredSol > availableSol) {
            throw new Error(`SOL 잔액이 부족합니다. (필요: ${requiredSol.toFixed(4)} SOL, 보유: ${availableSol.toFixed(4)} SOL)`);
          }
        }
      }

      // 주문 요청 생성
      const request: CreateOrderRequest = {
        order_type: orderType,
        order_side: orderSide,
        base_mint: 'SOL',
        quote_mint: 'USDT',
      };

      if (orderSide === 'limit') {
        request.price = price;
        request.amount = amount;
      } else { // market
        if (orderType === 'buy') {
          request.quote_amount = quoteAmount;
        } else { // sell
          request.amount = amount;
        }
      }
      
      console.log('주문 요청 데이터:', JSON.stringify(request, null, 2));
      console.log('주문 타입:', orderType, '주문 방식:', orderSide);
      
      try {
        const order = await apiClient.createOrder(request);
      
        const successMessage = `${orderType === 'buy' ? '매수' : '매도'} 주문이 생성되었습니다. (ID: ${order.id})`;
        setSuccess(successMessage);
        showAlert(successMessage, 'success');
        
        // 폼 초기화
        setPrice('');
        setAmount('');
        setQuoteAmount('');
        setTotal('');

        // 잔액 갱신
        const response = await apiClient.getBalances();
        setBalances(response.balances);
      } catch (orderError) {
        console.error('주문 생성 실패 (상세):', orderError);
        throw orderError; // 상위 catch로 전달
      }
    } catch (err) {
      console.error('주문 생성 실패:', err);
      let errorMessage = '주문 생성에 실패했습니다.';
      if (err instanceof Error) {
        errorMessage = err.message;
        
        // 백엔드 엔진 에러 처리
        if (err.message.includes('Failed to submit order to engine')) {
          errorMessage = '주문 처리 중 오류가 발생했습니다. 잠시 후 다시 시도해주세요.';
          console.error('엔진 제출 실패 - 백엔드 로그 확인 필요');
        } else if (err.message.includes('Insufficient balance')) {
          // 잔액 부족 에러 파싱 (여러 형식 지원)
          // 형식 1: "Insufficient balance: required 2500000.00, but only 2255292.54 available"
          // 형식 2: "Failed to create order: Insufficient balance: required 2500000.00, but only 2255292.54 available"
          const match = err.message.match(/required ([\d.]+), but only ([\d.]+) available/);
          if (match) {
            const required = parseFloat(match[1]);
            const available = parseFloat(match[2]);
            const mint = orderType === 'buy' ? 'USDT' : 'SOL';
            errorMessage = `잔액이 부족합니다. (필요: ${required.toLocaleString('ko-KR', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ${mint}, 보유: ${available.toLocaleString('ko-KR', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ${mint})`;
          } else {
            // 파싱 실패 시 원본 메시지 사용
            errorMessage = err.message.includes('잔액이 부족합니다') 
              ? err.message 
              : '잔액이 부족합니다.';
          }
        } else if (err.message.includes('Failed to create order')) {
          // 기타 백엔드 에러
          const match = err.message.match(/Failed to create order: (.+)/);
          if (match) {
            errorMessage = `주문 생성 실패: ${match[1]}`;
          } else {
            errorMessage = err.message;
          }
        }
      }
      setError(errorMessage);
      showAlert(errorMessage, 'error');
    } finally {
      setLoading(false);
    }
  };

  const solBalance = getBalance('SOL');
  const usdtBalance = getBalance('USDT');

  return (
    <div className="bg-gray-800 rounded-lg border border-gray-700 p-5 h-full flex flex-col">
      {/* 매수/매도 버튼 */}
      <div className="flex mb-3 flex-shrink-0">
        <button
          onClick={() => {
            setOrderType('buy');
            setError(null);
            setSuccess(null);
          }}
          className={`flex-1 py-2.5 px-4 rounded-l font-semibold text-sm transition-colors ${
            orderType === 'buy'
              ? 'bg-red-600 text-white'
              : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
          }`}
        >
          매수
        </button>
        <button
          onClick={() => {
            setOrderType('sell');
            setError(null);
            setSuccess(null);
          }}
          className={`flex-1 py-2.5 px-4 rounded-r font-semibold text-sm transition-colors ${
            orderType === 'sell'
              ? 'bg-blue-600 text-white'
              : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
          }`}
        >
          매도
        </button>
      </div>

      {/* 지정가/시장가 탭 */}
      <div className="flex mb-3 flex-shrink-0">
        <button
          onClick={() => {
            setOrderSide('limit');
            setError(null);
            setSuccess(null);
            setQuoteAmount(''); // 시장가 매수 금액 초기화
          }}
          className={`flex-1 py-1.5 px-3 rounded-l text-xs font-medium transition-colors ${
            orderSide === 'limit'
              ? 'bg-gray-700 text-white'
              : 'bg-gray-900 text-gray-400 hover:bg-gray-800'
          }`}
        >
          지정가
        </button>
        <button
          onClick={() => {
            setOrderSide('market');
            setError(null);
            setSuccess(null);
            setPrice(''); // 지정가 가격 초기화
            setAmount(''); // 지정가 수량 초기화
            setTotal(''); // 총액 초기화
          }}
          className={`flex-1 py-1.5 px-3 rounded-r text-xs font-medium transition-colors ${
            orderSide === 'market'
              ? 'bg-gray-700 text-white'
              : 'bg-gray-900 text-gray-400 hover:bg-gray-800'
          }`}
        >
          시장가
        </button>
      </div>

      {/* 잔액 표시 */}
      <div className="mb-3 pb-3 border-b border-gray-700 flex-shrink-0">
        <div className="flex justify-between text-xs mb-1">
          <span className="text-gray-400">보유 {orderType === 'buy' ? 'USDT' : 'SOL'}</span>
          <span className="text-white font-medium">
            {orderType === 'buy' 
              ? `${usdtBalance.toFixed(2)} USDT`
              : `${solBalance.toFixed(4)} SOL`}
          </span>
        </div>
      </div>

      <form onSubmit={handleSubmit} className="flex-1 flex flex-col space-y-3 justify-between">
        <div className="space-y-3">
          {/* 지정가 주문: 가격 입력 */}
          {orderSide === 'limit' && (
            <div>
              <label className="block text-sm text-gray-400 mb-1">가격 (USDT)</label>
              <div className="flex items-center gap-2">
                <input
                  type="number"
                  step="0.01"
                  value={price}
                  onChange={handlePriceChange}
                  placeholder="0.00"
                  className="w-full bg-gray-900 border border-gray-600 rounded px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                />
                <button
                  type="button"
                  onClick={handleSetCurrentPrice}
                  className="flex-shrink-0 bg-gray-700 hover:bg-gray-600 text-white text-xs px-3 py-2 rounded transition-colors"
                  disabled={!solPrice}
                >
                  현재가
                </button>
              </div>
            </div>
          )}

          {/* 수량 입력 (지정가 매수, 모든 매도) */}
          {(orderSide === 'limit' || orderType === 'sell') && (
            <div>
              <label className="block text-sm text-gray-400 mb-1">수량 (SOL)</label>
              <input
                type="number"
                step="0.0001"
                value={amount}
                onChange={handleAmountChange}
                placeholder="0.0000"
                className="w-full bg-gray-900 border border-gray-600 rounded px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
              />
              <div className="flex gap-2 mt-1.5">
                {[25, 50, 75, 100].map((percent) => (
                  <button
                    key={percent}
                    type="button"
                    onClick={() => setPercentage(percent)}
                    className="flex-1 py-1 px-2 bg-gray-700 text-gray-300 text-xs rounded hover:bg-gray-600"
                  >
                    {percent}%
                  </button>
                ))}
              </div>
            </div>
          )}

          {/* 금액 입력 (시장가 매수) */}
          {orderSide === 'market' && orderType === 'buy' && (
            <div>
              <label className="block text-sm text-gray-400 mb-1">금액 (USDT)</label>
              <input
                type="number"
                step="0.01"
                value={quoteAmount}
                onChange={handleQuoteAmountChange}
                placeholder="0.00"
                className="w-full bg-gray-900 border border-gray-600 rounded px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
              />
              <div className="flex gap-2 mt-1.5">
                {[25, 50, 75, 100].map((percent) => (
                  <button
                    key={percent}
                    type="button"
                    onClick={() => setPercentage(percent)}
                    className="flex-1 py-1 px-2 bg-gray-700 text-gray-300 text-xs rounded hover:bg-gray-600"
                  >
                    {percent}%
                  </button>
                ))}
              </div>
            </div>
          )}

          {/* 총액 */}
          <div>
            <label className="block text-sm text-gray-400 mb-1">총액 (USDT)</label>
            <input
              type="text"
              value={total}
              readOnly
              className="w-full bg-gray-900 border border-gray-600 rounded px-3 py-2 text-sm text-gray-400"
            />
          </div>
        </div>

        {error && <div className="text-red-400 text-sm text-center mt-2">{error}</div>}
        {success && <div className="text-green-400 text-sm text-center mt-2">{success}</div>}

        <button
          type="submit"
          className={`w-full py-2.5 rounded font-semibold transition-colors flex-shrink-0 ${
            orderType === 'buy'
              ? 'bg-red-600 hover:bg-red-700 text-white'
              : 'bg-blue-600 hover:bg-blue-700 text-white'
          } ${loading ? 'opacity-50 cursor-not-allowed' : ''}`}
          disabled={loading}
        >
          {loading ? '주문 처리 중...' : '주문하기'}
        </button>
      </form>
      <AlertContainer />
    </div>
  );
}
