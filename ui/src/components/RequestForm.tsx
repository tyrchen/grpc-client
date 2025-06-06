import { useState, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Badge } from '@/components/ui/badge';
import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Switch } from '@/components/ui/switch';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Play, RefreshCw, AlertCircle, Code, Info, Plus, Trash2, FileText, Settings } from 'lucide-react';
import { useAppStore, useSelectedServer, useSelectedMethod } from '@/lib/store';
import { apiClient } from '@/lib/api';

interface HeaderRow {
  id: string;
  key: string;
  value: string;
}

interface FormFieldProps {
  name: string;
  schema: any;
  value: any;
  onChange: (value: any) => void;
  required?: boolean;
}

function FormField({ name, schema, value, onChange, required }: FormFieldProps) {
  const renderField = () => {
    // Handle enum fields (string type with enumValues)
    if (schema.type === 'string' && schema.enumValues && Array.isArray(schema.enumValues)) {
      return (
        <Select value={value || ''} onValueChange={onChange}>
          <SelectTrigger className="text-xs">
            <SelectValue placeholder={`Select ${name}`} />
          </SelectTrigger>
          <SelectContent>
            {schema.enumValues.map((enumValue: string) => (
              <SelectItem key={enumValue} value={enumValue} className="text-xs">
                {enumValue}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      );
    }

    switch (schema.type) {
      case 'string':
        return (
          <Input
            type="text"
            value={value || ''}
            onChange={(e) => onChange(e.target.value)}
            placeholder={schema.example || `Enter ${name}`}
            className="text-xs"
          />
        );

      case 'number':
      case 'integer':
        return (
          <Input
            type="number"
            value={value || ''}
            onChange={(e) => onChange(e.target.value ? Number(e.target.value) : null)}
            placeholder={schema.example?.toString() || '0'}
            className="text-xs"
          />
        );

      case 'boolean':
        return (
          <div className="flex items-center space-x-2">
            <Switch
              checked={!!value}
              onCheckedChange={onChange}
            />
            <span className="text-xs text-gray-600">{value ? 'true' : 'false'}</span>
          </div>
        );

      case 'array':
        const arrayValue = Array.isArray(value) ? value : [];
        return (
          <div className="space-y-2">
            {arrayValue.map((item: any, index: number) => (
              <div key={index} className="flex items-center space-x-2">
                <Input
                  type="text"
                  value={typeof item === 'object' ? JSON.stringify(item) : item}
                  onChange={(e) => {
                    const newArray = [...arrayValue];
                    try {
                      newArray[index] = schema.items?.type === 'object' ? JSON.parse(e.target.value) : e.target.value;
                    } catch {
                      newArray[index] = e.target.value;
                    }
                    onChange(newArray);
                  }}
                  placeholder={`Item ${index + 1}`}
                  className="text-xs flex-1"
                />
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    const newArray = arrayValue.filter((_, i) => i !== index);
                    onChange(newArray);
                  }}
                  className="h-8 w-8 p-0"
                >
                  <Trash2 className="w-3 h-3" />
                </Button>
              </div>
            ))}
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                const newArray = [...arrayValue, schema.items?.type === 'string' ? '' : schema.items?.type === 'number' ? 0 : {}];
                onChange(newArray);
              }}
              className="gap-1 h-7"
            >
              <Plus className="w-3 h-3" />
              Add Item
            </Button>
          </div>
        );

      case 'object':
        if (schema.properties) {
          const objectValue = typeof value === 'object' && value !== null ? value : {};
          return (
            <div className="space-y-3 pl-4 border-l-2 border-gray-200 dark:border-gray-700">
              {Object.entries(schema.properties).map(([propName, propSchema]: [string, any]) => (
                <FormField
                  key={propName}
                  name={propName}
                  schema={propSchema}
                  value={objectValue[propName]}
                  onChange={(propValue) => {
                    onChange({
                      ...objectValue,
                      [propName]: propValue
                    });
                  }}
                  required={schema.required?.includes(propName)}
                />
              ))}
            </div>
          );
        }
        // Fallback for object without defined properties
        return (
          <Textarea
            value={typeof value === 'object' ? JSON.stringify(value, null, 2) : value || '{}'}
            onChange={(e) => {
              try {
                onChange(JSON.parse(e.target.value));
              } catch {
                onChange(e.target.value);
              }
            }}
            placeholder="{}"
            className="font-mono text-xs h-20"
          />
        );

      default:
        return (
          <Input
            type="text"
            value={typeof value === 'object' ? JSON.stringify(value) : value || ''}
            onChange={(e) => {
              try {
                onChange(JSON.parse(e.target.value));
              } catch {
                onChange(e.target.value);
              }
            }}
            placeholder={`Enter ${name}`}
            className="text-xs"
          />
        );
    }
  };

  return (
    <div className="space-y-2">
      <div className="flex items-center gap-2">
        <Label className="text-sm font-medium">
          {name}
          {required && <span className="text-red-500 ml-1">*</span>}
        </Label>
        {schema.description && (
          <span className="text-xs text-gray-500">({schema.description})</span>
        )}
      </div>
      {renderField()}
    </div>
  );
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

  const [formData, setFormData] = useState<any>({});
  const [requestData, setRequestData] = useState('{}');
  const [headerRows, setHeaderRows] = useState<HeaderRow[]>([
    { id: '1', key: '', value: '' }
  ]);
  const [emitDefaults, setEmitDefaults] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<'form' | 'json'>('form');

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

  // Sync form data with JSON data
  useEffect(() => {
    if (viewMode === 'form') {
      try {
        setRequestData(JSON.stringify(formData, null, 2));
      } catch {
        // If formData is invalid, keep current requestData
      }
    }
  }, [formData, viewMode]);

  useEffect(() => {
    if (viewMode === 'json') {
      try {
        const parsed = JSON.parse(requestData);
        setFormData(parsed);
      } catch {
        // If requestData is invalid JSON, keep current formData
      }
    }
  }, [requestData, viewMode]);

  // Load method schema when method is selected
  useEffect(() => {
    const loadMethodSchema = async () => {
      if (!selectedServerId || !selectedServiceName || !selectedMethodName) {
        setMethodSchema(null);
        setFormData({});
        setRequestData('{}');
        return;
      }

      try {
        const schema = await apiClient.getMethodSchema(
          selectedServerId,
          selectedServiceName,
          selectedMethodName
        );
        setMethodSchema(schema);

        // Generate initial form data based on schema
        if (schema.schema) {
          const initialData = generateInitialFromSchema(schema.schema);
          setFormData(initialData);
          setRequestData(JSON.stringify(initialData, null, 2));
        }
      } catch (err) {
        console.error('Failed to load method schema:', err);
        setMethodSchema(null);
        setFormData({});
        setRequestData('{}');
      }
    };

    loadMethodSchema();
  }, [selectedServerId, selectedServiceName, selectedMethodName, setMethodSchema]);

  const generateInitialFromSchema = (schema: any): any => {
    if (!schema || !schema.properties) {
      return {};
    }

    const initial: any = {};
    Object.entries(schema.properties).forEach(([key, prop]: [string, any]) => {
      const isRequired = schema.required?.includes(key);

      switch (prop.type) {
        case 'string':
          initial[key] = prop.example || (isRequired ? '' : undefined);
          break;
        case 'number':
        case 'integer':
          initial[key] = prop.example ?? (isRequired ? 0 : undefined);
          break;
        case 'boolean':
          initial[key] = prop.example ?? (isRequired ? false : undefined);
          break;
        case 'array':
          initial[key] = isRequired ? [] : undefined;
          break;
        case 'object':
          if (prop.properties) {
            initial[key] = isRequired ? generateInitialFromSchema(prop) : undefined;
          } else {
            initial[key] = isRequired ? {} : undefined;
          }
          break;
        default:
          initial[key] = isRequired ? null : undefined;
      }
    });

    // Remove undefined values
    Object.keys(initial).forEach(key => {
      if (initial[key] === undefined) {
        delete initial[key];
      }
    });

    return initial;
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

      // Use form data if in form mode, otherwise parse JSON
      let parsedData;
      if (viewMode === 'form') {
        parsedData = formData;
      } else {
        try {
          parsedData = JSON.parse(requestData);
        } catch (e) {
          throw new Error('Invalid JSON in request data');
        }
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
              {methodSchema && (
                <>
                  <div><strong>Input:</strong> {methodSchema.input_type}</div>
                  <div><strong>Output:</strong> {methodSchema.output_type}</div>
                </>
              )}
            </div>
          </div>

          {methodSchema && (
            <div className="flex gap-2 items-center">
              <Badge variant="outline" className="text-xs">
                {methodSchema.streaming_type}
              </Badge>
              {selectedMethod?.clientStreaming && (
                <Badge variant="secondary" className="text-xs">Client Stream</Badge>
              )}
              {selectedMethod?.serverStreaming && (
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
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Schema Info */}
          {methodSchema && (
            <div className="p-3 bg-blue-50 rounded-md border border-blue-200 dark:bg-blue-900/20 dark:border-blue-700">
              <div className="flex gap-2 items-center text-xs text-blue-700 dark:text-blue-300">
                <Info className="w-3 h-3" />
                <span>Schema loaded - form fields generated automatically</span>
              </div>
            </div>
          )}

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

          {/* Request Input - Form or JSON */}
          {methodSchema?.schema ? (
            <Tabs value={viewMode} onValueChange={(value) => setViewMode(value as 'form' | 'json')}>
              <TabsList className="grid w-full grid-cols-2">
                <TabsTrigger value="form" className="gap-2">
                  <Settings className="w-3 h-3" />
                  Form
                </TabsTrigger>
                <TabsTrigger value="json" className="gap-2">
                  <FileText className="w-3 h-3" />
                  JSON
                </TabsTrigger>
              </TabsList>

              <TabsContent value="form" className="space-y-4 mt-4">
                {methodSchema.schema.properties ? (
                  <div className="space-y-4">
                    {Object.entries(methodSchema.schema.properties).map(([fieldName, fieldSchema]: [string, any]) => (
                      <FormField
                        key={fieldName}
                        name={fieldName}
                        schema={fieldSchema}
                        value={formData[fieldName]}
                        onChange={(value) => {
                          setFormData((prev: any) => ({
                            ...prev,
                            [fieldName]: value
                          }));
                        }}
                        required={methodSchema.schema.required?.includes(fieldName)}
                      />
                    ))}
                  </div>
                ) : (
                  <div className="p-4 text-center text-gray-500 text-sm">
                    No schema properties available
                  </div>
                )}
              </TabsContent>

              <TabsContent value="json" className="mt-4">
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
              </TabsContent>
            </Tabs>
          ) : (
            /* Fallback JSON input when no schema */
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
          )}

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
