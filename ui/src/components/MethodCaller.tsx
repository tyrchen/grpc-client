import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Badge } from '@/components/ui/badge';
import { Label } from '@/components/ui/label';
import { Play, Copy, RefreshCw, AlertCircle, CheckCircle } from 'lucide-react';
import { useAppStore, useSelectedServer, useSelectedMethod } from '@/lib/store';
import { apiClient } from '@/lib/api';

export function MethodCaller() {
  const {
    selectedServerId,
    selectedServiceName,
    selectedMethodName,
    callInProgress,
    setCallInProgress,
    addCallToHistory,
  } = useAppStore();

  const selectedServer = useSelectedServer();
  const selectedMethod = useSelectedMethod();

  const [requestData, setRequestData] = useState('{}');
  const [response, setResponse] = useState<any>(null);
  const [error, setError] = useState<string | null>(null);

  const handleCall = async () => {
    if (!selectedServerId || !selectedServiceName || !selectedMethodName) {
      setError('Please select a server, service, and method');
      return;
    }

    try {
      setCallInProgress(true);
      setError(null);

      // Parse the request data
      let parsedData;
      try {
        parsedData = JSON.parse(requestData);
      } catch (e) {
        throw new Error('Invalid JSON in request data');
      }

      const startTime = Date.now();

      // Build the method name in the format expected by the backend
      const methodPath = `${selectedServiceName}/${selectedMethodName}`;

      const result = await apiClient.callMethod(selectedServerId, {
        method: methodPath,
        data: parsedData,
        headers: {},
        emitDefaults: false,
      });

      const duration = Date.now() - startTime;

      setResponse(result);

      // Add to call history
      addCallToHistory({
        id: `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
        timestamp: new Date().toISOString(),
        serverId: selectedServerId,
        serviceName: selectedServiceName,
        methodName: selectedMethodName,
        request: parsedData,
        response: result,
        duration,
      });

    } catch (err) {
      setError(err instanceof Error ? err.message : 'Call failed');
      setResponse(null);
    } finally {
      setCallInProgress(false);
    }
  };

  const copyResponse = () => {
    if (response) {
      navigator.clipboard.writeText(JSON.stringify(response, null, 2));
    }
  };

  const formatExample = () => {
    if (selectedMethod) {
      // Generate a simple example based on the input type
      const exampleData = {};
      setRequestData(JSON.stringify(exampleData, null, 2));
    }
  };

  if (!selectedServerId || !selectedServiceName || !selectedMethodName) {
    return (
      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        {/* Request Panel */}
        <div className="space-y-4">
          <div className="p-4 bg-white rounded-lg border dark:bg-gray-800">
            <h3 className="mb-4 text-lg font-semibold">Request</h3>
            <div className="p-8 text-center text-gray-500 bg-gray-50 rounded-lg">
              <AlertCircle className="mx-auto mb-3 w-12 h-12 text-gray-400" />
              <p className="text-sm">Select a method from the Explorer tab to make a call</p>
            </div>
          </div>
        </div>

        {/* Response Panel */}
        <div className="space-y-4">
          <div className="p-4 bg-white rounded-lg border dark:bg-gray-800">
            <h3 className="mb-4 text-lg font-semibold">Response</h3>
            <div className="p-8 text-center text-gray-500 bg-gray-50 rounded-lg">
              <div className="mx-auto mb-3 w-12 h-12 bg-gray-300 rounded"></div>
              <p className="text-sm">Response will appear here after making a call</p>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
      {/* Request Panel */}
      <div className="space-y-4">
        <div className="p-4 bg-white rounded-lg border dark:bg-gray-800">
          <div className="flex justify-between items-center mb-4">
            <h3 className="text-lg font-semibold">Request</h3>
            <div className="flex items-center space-x-2">
              <Button
                variant="outline"
                size="sm"
                onClick={formatExample}
                className="gap-2"
              >
                <RefreshCw className="w-3 h-3" />
                Example
              </Button>
            </div>
          </div>

          {/* Method Info */}
          <div className="p-3 mb-4 bg-blue-50 rounded-md border border-blue-200">
            <div className="flex justify-between items-center mb-2">
              <span className="font-medium text-blue-900">
                {selectedServiceName}.{selectedMethodName}
              </span>
              {selectedMethod && (
                <Badge variant="outline" className="text-xs">
                  {selectedMethod.streamingType}
                </Badge>
              )}
            </div>
            <div className="text-xs text-blue-700">
              <div><strong>Server:</strong> {selectedServer?.name}</div>
              {selectedMethod && (
                <>
                  <div><strong>Input:</strong> {selectedMethod.inputType}</div>
                  <div><strong>Output:</strong> {selectedMethod.outputType}</div>
                </>
              )}
            </div>
          </div>

          {/* Request Data */}
          <div className="space-y-2">
            <Label htmlFor="request-data">Request Data (JSON)</Label>
            <Textarea
              id="request-data"
              value={requestData}
              onChange={(e) => setRequestData(e.target.value)}
              placeholder='{"key": "value"}'
              className="font-mono text-sm min-h-[200px]"
            />
          </div>

          {/* Call Button */}
          <div className="flex justify-end pt-4">
            <Button
              onClick={handleCall}
              disabled={callInProgress}
              className="gap-2"
            >
              {callInProgress ? (
                <RefreshCw className="w-4 h-4 animate-spin" />
              ) : (
                <Play className="w-4 h-4" />
              )}
              {callInProgress ? 'Calling...' : 'Call Method'}
            </Button>
          </div>
        </div>
      </div>

      {/* Response Panel */}
      <div className="space-y-4">
        <div className="p-4 bg-white rounded-lg border dark:bg-gray-800">
          <div className="flex justify-between items-center mb-4">
            <h3 className="text-lg font-semibold">Response</h3>
            {response && (
              <Button
                variant="outline"
                size="sm"
                onClick={copyResponse}
                className="gap-2"
              >
                <Copy className="w-3 h-3" />
                Copy
              </Button>
            )}
          </div>

          {error && (
            <div className="p-3 mb-4 bg-red-50 rounded-md border border-red-200">
              <div className="flex items-center space-x-2">
                <AlertCircle className="w-4 h-4 text-red-500" />
                <span className="text-sm text-red-700">{error}</span>
              </div>
            </div>
          )}

          {response ? (
            <div className="space-y-3">
              {/* Response Status */}
              <div className="flex items-center space-x-2">
                <CheckCircle className="w-4 h-4 text-green-500" />
                <span className="text-sm font-medium text-green-700">
                  {response.success ? 'Success' : 'Failed'}
                </span>
                {response.response && (
                  <Badge variant="outline" className="text-xs">
                    {response.response.length} response(s)
                  </Badge>
                )}
              </div>

              {/* Response Data */}
              <div className="relative">
                <Label>Response Data</Label>
                <pre className="mt-1 p-3 bg-gray-50 border rounded-md text-xs font-mono overflow-auto max-h-[400px]">
                  {JSON.stringify(response, null, 2)}
                </pre>
              </div>
            </div>
          ) : (
            <div className="p-8 text-center text-gray-500 bg-gray-50 rounded-lg">
              <div className="mx-auto mb-3 w-12 h-12 bg-gray-300 rounded"></div>
              <p className="text-sm">Response will appear here after making a call</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
