import { useEffect } from 'react';
import { Button } from './ui/button';
import { Badge } from './ui/badge';
import { RefreshCw, Zap, Loader2, AlertCircle, Play, Code } from 'lucide-react';
import { useAppStore } from '../lib/store';
import { apiClient } from '../lib/api';

export function MethodList() {
  const {
    selectedServerId,
    selectedServiceName,
    methods,
    selectedMethodName,
    loading,
    error,
    setMethods,
    setSelectedMethodName,
    setLoading,
    setError,
  } = useAppStore();

  const loadMethods = async () => {
    if (!selectedServerId || !selectedServiceName) return;

    try {
      setLoading('methods', true);
      setError(null);
      const methods = await apiClient.describeService(selectedServerId, selectedServiceName);

      // Extract methods from the description response
      if (methods.description && methods.description.methods) {
        setMethods(methods.description.methods || []);
      } else {
        setMethods([]);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load methods');
      setMethods([]);
    } finally {
      setLoading('methods', false);
    }
  };

  useEffect(() => {
    if (selectedServerId && selectedServiceName) {
      loadMethods();
    } else {
      setMethods([]);
      setSelectedMethodName(null);
    }
  }, [selectedServerId, selectedServiceName]);

  const handleMethodSelect = (methodName: string) => {
    setSelectedMethodName(methodName === selectedMethodName ? null : methodName);
  };

  const getStreamingBadge = (method: any) => {
    if (method.clientStreaming && method.serverStreaming) {
      return <Badge variant="default" className="text-xs bg-purple-100 text-purple-700">Bidirectional</Badge>;
    } else if (method.clientStreaming) {
      return <Badge variant="default" className="text-xs bg-blue-100 text-blue-700">Client Stream</Badge>;
    } else if (method.serverStreaming) {
      return <Badge variant="default" className="text-xs bg-green-100 text-green-700">Server Stream</Badge>;
    } else {
      return <Badge variant="outline" className="text-xs">Unary</Badge>;
    }
  };

  if (!selectedServerId || !selectedServiceName) {
    return (
      <div className="space-y-4">
        <div className="flex items-center space-x-2">
          <Zap className="w-5 h-5 text-gray-400" />
          <h3 className="text-lg font-semibold text-gray-400">Methods</h3>
        </div>
        <div className="p-4 text-center text-gray-500 bg-gray-50 rounded-lg">
          Select a service to view methods
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-2">
          <Zap className="w-5 h-5 text-gray-600" />
          <h3 className="text-lg font-semibold">Methods</h3>
          {selectedServiceName && (
            <Badge variant="outline" className="text-xs">
              {selectedServiceName}
            </Badge>
          )}
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={loadMethods}
          disabled={loading.methods}
        >
          {loading.methods ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <RefreshCw className="w-4 h-4" />
          )}
        </Button>
      </div>

      {error && (
        <div className="p-3 bg-red-50 border border-red-200 rounded-md">
          <div className="flex items-center space-x-2">
            <AlertCircle className="w-4 h-4 text-red-500" />
            <span className="text-sm text-red-700">{error}</span>
          </div>
        </div>
      )}

      {loading.methods ? (
        <div className="space-y-2">
          {[1, 2, 3].map((i) => (
            <div key={i} className="p-3 bg-gray-100 rounded-md animate-pulse">
              <div className="h-4 bg-gray-300 rounded w-2/3 mb-2"></div>
              <div className="h-3 bg-gray-300 rounded w-1/2"></div>
            </div>
          ))}
        </div>
      ) : methods.length > 0 ? (
        <div className="space-y-2">
          {methods.map((method) => (
            <div
              key={method.name}
              className={`p-3 border rounded-md cursor-pointer transition-colors ${selectedMethodName === method.name
                ? 'bg-green-50 border-green-200'
                : 'bg-white border-gray-200 hover:bg-gray-50'
                }`}
              onClick={() => handleMethodSelect(method.name)}
            >
              <div className="flex items-start justify-between">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center space-x-2 mb-1">
                    <span className="font-medium text-gray-900 truncate">
                      {method.name}
                    </span>
                    {getStreamingBadge(method)}
                  </div>

                  <div className="text-xs text-gray-600 space-y-1">
                    <div>
                      <span className="font-medium">Input:</span> {method.inputType}
                    </div>
                    <div>
                      <span className="font-medium">Output:</span> {method.outputType}
                    </div>
                  </div>

                  {method.description && (
                    <p className="text-sm text-gray-600 mt-2">
                      {method.description}
                    </p>
                  )}
                </div>

                {selectedMethodName === method.name && (
                  <div className="flex space-x-1 ml-2">
                    <Button size="sm" variant="outline" className="h-7 px-2">
                      <Code className="w-3 h-3" />
                    </Button>
                    <Button size="sm" variant="default" className="h-7 px-2">
                      <Play className="w-3 h-3" />
                    </Button>
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="p-4 text-center text-gray-500 bg-gray-50 rounded-lg">
          No methods found
        </div>
      )}

      {selectedMethodName && (
        <div className="p-3 bg-green-50 border border-green-200 rounded-md">
          <p className="text-sm text-green-700">
            <strong>Selected:</strong> {selectedMethodName}
          </p>
          <p className="text-xs text-green-600 mt-1">
            Ready to call this method
          </p>
        </div>
      )}
    </div>
  );
}
