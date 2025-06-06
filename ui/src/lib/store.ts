import { create } from 'zustand';
import { devtools } from 'zustand/middleware';
import { ServerStatus, ServiceInfo, MethodInfo, MethodSchema, CallHistoryEntry } from './api';

export interface AppState {
  // Server management
  servers: ServerStatus[];
  selectedServerId: string | null;

  // Service management
  services: ServiceInfo[];
  selectedServiceName: string | null;

  // Method management
  methods: MethodInfo[];
  selectedMethodName: string | null;

  // Method schema and call state
  methodSchema: MethodSchema | null;
  callInProgress: boolean;
  callHistory: CallHistoryEntry[];

  // UI state
  sidebarCollapsed: boolean;
  theme: 'light' | 'dark';

  // Loading states
  loading: {
    servers: boolean;
    services: boolean;
    methods: boolean;
    schema: boolean;
    call: boolean;
  };

  // Error state
  error: string | null;
}

export interface AppActions {
  // Server actions
  setServers: (servers: ServerStatus[]) => void;
  setSelectedServerId: (serverId: string | null) => void;

  // Service actions
  setServices: (services: ServiceInfo[]) => void;
  setSelectedServiceName: (serviceName: string | null) => void;

  // Method actions
  setMethods: (methods: MethodInfo[]) => void;
  setSelectedMethodName: (methodName: string | null) => void;

  // Schema and call actions
  setMethodSchema: (schema: MethodSchema | null) => void;
  setCallInProgress: (inProgress: boolean) => void;
  addCallToHistory: (entry: CallHistoryEntry) => void;
  clearCallHistory: () => void;

  // UI actions
  setSidebarCollapsed: (collapsed: boolean) => void;
  setTheme: (theme: 'light' | 'dark') => void;

  // Loading actions
  setLoading: (key: keyof AppState['loading'], loading: boolean) => void;

  // Error actions
  setError: (error: string | null) => void;

  // Reset actions
  resetSelection: () => void;
  resetServiceSelection: () => void;
  resetMethodSelection: () => void;
}

const initialState: AppState = {
  servers: [],
  selectedServerId: null,
  services: [],
  selectedServiceName: null,
  methods: [],
  selectedMethodName: null,
  methodSchema: null,
  callInProgress: false,
  callHistory: [],
  sidebarCollapsed: false,
  theme: 'light',
  loading: {
    servers: false,
    services: false,
    methods: false,
    schema: false,
    call: false,
  },
  error: null,
};

export const useAppStore = create<AppState & AppActions>()(
  devtools(
    (set) => ({
      ...initialState,

      // Server actions
      setServers: (servers: ServerStatus[]) =>
        set((state) => ({
          ...state,
          servers,
        })),

      setSelectedServerId: (serverId: string | null) =>
        set((state) => ({
          ...state,
          selectedServerId: serverId,
          // Reset downstream selections when server changes
          selectedServiceName: null,
          selectedMethodName: null,
          services: [],
          methods: [],
          methodSchema: null,
        })),

      // Service actions
      setServices: (services: ServiceInfo[]) =>
        set((state) => ({
          ...state,
          services,
        })),

      setSelectedServiceName: (serviceName: string | null) =>
        set((state) => ({
          ...state,
          selectedServiceName: serviceName,
          // Reset downstream selections when service changes
          selectedMethodName: null,
          methods: [],
          methodSchema: null,
        })),

      // Method actions
      setMethods: (methods: MethodInfo[]) =>
        set((state) => ({
          ...state,
          methods,
        })),

      setSelectedMethodName: (methodName: string | null) =>
        set((state) => ({
          ...state,
          selectedMethodName: methodName,
          methodSchema: null,
        })),

      // Schema and call actions
      setMethodSchema: (schema: MethodSchema | null) =>
        set((state) => ({
          ...state,
          methodSchema: schema,
        })),

      setCallInProgress: (inProgress: boolean) =>
        set((state) => ({
          ...state,
          callInProgress: inProgress,
        })),

      addCallToHistory: (entry: CallHistoryEntry) =>
        set((state) => ({
          ...state,
          callHistory: [entry, ...state.callHistory.slice(0, 99)], // Keep only last 100
        })),

      clearCallHistory: () =>
        set((state) => ({
          ...state,
          callHistory: [],
        })),

      // UI actions
      setSidebarCollapsed: (collapsed: boolean) =>
        set((state) => ({
          ...state,
          sidebarCollapsed: collapsed,
        })),

      setTheme: (theme: 'light' | 'dark') =>
        set((state) => ({
          ...state,
          theme,
        })),

      // Loading actions
      setLoading: (key: keyof AppState['loading'], loading: boolean) =>
        set((state) => ({
          ...state,
          loading: {
            ...state.loading,
            [key]: loading,
          },
        })),

      // Error actions
      setError: (error: string | null) =>
        set((state) => ({
          ...state,
          error,
        })),

      // Reset actions
      resetSelection: () =>
        set((state) => ({
          ...state,
          selectedServerId: null,
          selectedServiceName: null,
          selectedMethodName: null,
          services: [],
          methods: [],
          methodSchema: null,
        })),

      resetServiceSelection: () =>
        set((state) => ({
          ...state,
          selectedServiceName: null,
          selectedMethodName: null,
          methods: [],
          methodSchema: null,
        })),

      resetMethodSelection: () =>
        set((state) => ({
          ...state,
          selectedMethodName: null,
          methodSchema: null,
        })),
    }),
    {
      name: 'grpc-client-store',
    }
  )
);

// Selectors for computed values
export const useSelectedServer = () => {
  const { servers, selectedServerId } = useAppStore();
  return servers.find(s => s.id === selectedServerId) || null;
};

export const useSelectedService = () => {
  const { services, selectedServiceName } = useAppStore();
  return services.find(s => s.name === selectedServiceName) || null;
};

export const useSelectedMethod = () => {
  const { methods, selectedMethodName } = useAppStore();
  return methods.find(m => m.name === selectedMethodName) || null;
};
