import { useState, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Badge } from '@/components/ui/badge';
import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Play, RefreshCw, AlertCircle, Code, Info, Plus, Trash2 } from 'lucide-react';
import { useAppStore, useSelectedServer, useSelectedMethod } from '@/lib/store';
import { apiClient } from '@/lib/api';

interface HeaderRow {
  id: string;
  key: string;
  value: string;
}

export function RequestForm() {
  const {
    selectedServerId,
    selectedServiceName,
    selectedMethodName,
    methodSchema,
    callInProgress,
    setCallInProgress,
    setMethodSchema,
    addCallToHistory,
  } = useAppStore();

  const selectedServer = useSelectedServer();
  const selectedMethod = useSelectedMethod();

  const [requestData, setRequestData] = useState('{}');
  const [headerRows, setHeaderRows] = useState<HeaderRow[]>([
    { id: '1', key: '', value: '' }
  ]);
  const [emitDefaults, setEmitDefaults] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Convert header rows to object
  const getHeadersObject = () => {
    const headers: Record<string, string> = {};
    headerRows.forEach(row => {
      if (row.key.trim() && row.value.trim()) {
        headers[row.key.trim()] = row.value.trim();
      }
    });
    return headers;
  };

  // Load method schema when method is selected
  useEffect(() => {
    const loadMethodSchema = async () => {
      if (!selectedServerId || !selectedServiceName || !selectedMethodName) {
        setMethodSchema(null);
        return;
      }

      try {
        const schema = await apiClient.getMethodSchema(
          selectedServerId,
          selectedServiceName,
          selectedMethodName
        );
        setMethodSchema(schema);

        // Generate example request based on schema
        if (schema.requestSchema) {
          const exampleData = generateExampleFromSchema(schema.requestSchema);
          setRequestData(JSON.stringify(exampleData, null, 2));
        }
      } catch (err) {
        console.error('Failed to load method schema:', err);
        setMethodSchema(null);
      }
    };

    loadMethodSchema();
  }, [selectedServerId, selectedServiceName, selectedMethodName, setMethodSchema]);

  const generateExampleFromSchema = (schema: any): any => {
    // Simple example generation - can be enhanced
    if (!schema || !schema.properties) {
      return {};
    }

    const example: any = {};
    Object.entries(schema.properties).forEach(([key, prop]: [string, any]) => {
      switch (prop.type) {
        case 'string':
          example[key] = prop.example || `example_${key}`;
          break;
        case 'number':
        case 'integer':
          example[key] = prop.example || 0;
          break;
        case 'boolean':
          example[key] = prop.example || false;
          break;
        case 'array':
          example[key] = [];
          break;
        case 'object':
          example[key] = prop.properties ? generateExampleFromSchema(prop) : {};
          break;
        default:
          example[key] = null;
      }
    });

    return example;
  };

  const addHeaderRow = () => {
    const newRow: HeaderRow = {
      id: Date.now().toString(),
      key: '',
      value: ''
    };
    setHeaderRows([...headerRows, newRow]);
  };

  const removeHeaderRow = (id: string) => {
    if (headerRows.length > 1) {
      setHeaderRows(headerRows.filter(row => row.id !== id));
    }
  };

  const updateHeaderRow = (id: string, field: 'key' | 'value', value: string) => {
    setHeaderRows(headerRows.map(row =>
      row.id === id ? { ...row, [field]: value } : row
    ));
  };

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

      const headersObj = getHeadersObject();

      const result = await apiClient.callMethod(selectedServerId, {
        method: methodPath,
        data: parsedData,
        headers: headersObj,
        emitDefaults,
      });

      const duration = Date.now() - startTime;

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
        headers: headersObj,
      });

    } catch (err) {
      setError(err instanceof Error ? err.message : 'Call failed');
    } finally {
      setCallInProgress(false);
    }
  };

  const formatExample = () => {
    if (methodSchema && methodSchema.requestSchema) {
      const exampleData = generateExampleFromSchema(methodSchema.requestSchema);
      setRequestData(JSON.stringify(exampleData, null, 2));
    } else {
      setRequestData('{}');
    }
  };

  if (!selectedServerId || !selectedServiceName || !selectedMethodName) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex gap-2 items-center">
            <Code className="w-5 h-5" />
            Request Builder
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="p-8 text-center text-gray-500">
            <AlertCircle className="mx-auto mb-3 w-12 h-12 text-gray-400" />
            <p className="text-sm">Select a method to build a request</p>
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-4">
      {/* Method Info */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="flex gap-2 items-center text-base">
            <Code className="w-4 h-4" />
            Method Details
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="text-sm">
            <div className="font-medium text-blue-900 dark:text-blue-100">
              {selectedServiceName}.{selectedMethodName}
            </div>
            <div className="mt-1 text-xs text-gray-600 dark:text-gray-400">
              <div><strong>Server:</strong> {selectedServer?.name}</div>
              {selectedMethod && (
                <>
                  <div><strong>Input:</strong> {selectedMethod.inputType}</div>
                  <div><strong>Output:</strong> {selectedMethod.outputType}</div>
                </>
              )}
            </div>
          </div>

          {selectedMethod && (
            <div className="flex gap-2 items-center">
              <Badge variant="outline" className="text-xs">
                {selectedMethod.streamingType}
              </Badge>
              {selectedMethod.clientStreaming && (
                <Badge variant="secondary" className="text-xs">Client Stream</Badge>
              )}
              {selectedMethod.serverStreaming && (
                <Badge variant="secondary" className="text-xs">Server Stream</Badge>
              )}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Request Form */}
      <Card>
        <CardHeader className="pb-3">
          <div className="flex justify-between items-center">
            <CardTitle className="text-base">Request Data</CardTitle>
            <Button
              variant="outline"
              size="sm"
              onClick={formatExample}
              className="gap-1 h-7"
            >
              <RefreshCw className="w-3 h-3" />
              Example
            </Button>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Schema Info */}
          {methodSchema && (
            <div className="p-3 bg-blue-50 rounded-md border border-blue-200 dark:bg-blue-900/20 dark:border-blue-700">
              <div className="flex gap-2 items-center text-xs text-blue-700 dark:text-blue-300">
                <Info className="w-3 h-3" />
                <span>Schema loaded - use Example button to generate sample data</span>
              </div>
            </div>
          )}

          {/* Request JSON */}
          <div className="space-y-2">
            <Label htmlFor="request-data" className="text-sm">Request (JSON)</Label>
            <Textarea
              id="request-data"
              value={requestData}
              onChange={(e) => setRequestData(e.target.value)}
              placeholder='{"key": "value"}'
              className="font-mono text-xs min-h-[200px]"
            />
          </div>

          {/* Headers */}
          <div className="space-y-2">
            <div className="flex justify-between items-center">
              <Label className="text-sm">Headers</Label>
              <Button
                variant="outline"
                size="sm"
                onClick={addHeaderRow}
                className="gap-1 h-7"
              >
                <Plus className="w-3 h-3" />
                Add
              </Button>
            </div>

            <div className="space-y-2">
              {/* Header row labels */}
              <div className="grid grid-cols-12 gap-2 text-xs font-medium text-gray-500">
                <div className="col-span-5">KEY</div>
                <div className="col-span-6">VALUE</div>
                <div className="col-span-1"></div>
              </div>

              {/* Header input rows */}
              {headerRows.map((row) => (
                <div key={row.id} className="grid grid-cols-12 gap-2">
                  <Input
                    placeholder="Key"
                    value={row.key}
                    onChange={(e) => updateHeaderRow(row.id, 'key', e.target.value)}
                    className="col-span-5 h-8 text-xs"
                  />
                  <Input
                    placeholder="Value"
                    value={row.value}
                    onChange={(e) => updateHeaderRow(row.id, 'value', e.target.value)}
                    className="col-span-6 h-8 text-xs"
                  />
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => removeHeaderRow(row.id)}
                    disabled={headerRows.length === 1}
                    className="col-span-1 p-0 w-8 h-8"
                  >
                    <Trash2 className="w-3 h-3" />
                  </Button>
                </div>
              ))}
            </div>
          </div>

          {/* Options */}
          <div className="flex items-center space-x-2">
            <input
              type="checkbox"
              id="emit-defaults"
              checked={emitDefaults}
              onChange={(e) => setEmitDefaults(e.target.checked)}
              className="rounded"
            />
            <Label htmlFor="emit-defaults" className="text-sm">Emit default values</Label>
          </div>

          {/* Error */}
          {error && (
            <div className="p-3 bg-red-50 rounded-md border border-red-200 dark:bg-red-900/20 dark:border-red-700">
              <div className="flex items-center space-x-2">
                <AlertCircle className="w-4 h-4 text-red-500" />
                <span className="text-sm text-red-700 dark:text-red-300">{error}</span>
              </div>
            </div>
          )}

          {/* Call Button */}
          <Button
            onClick={handleCall}
            disabled={callInProgress}
            className="gap-2 w-full"
          >
            {callInProgress ? (
              <RefreshCw className="w-4 h-4 animate-spin" />
            ) : (
              <Play className="w-4 h-4" />
            )}
            {callInProgress ? 'Calling...' : 'Call Method'}
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
