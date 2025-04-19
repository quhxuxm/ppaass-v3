import {LogLevel} from "./common.ts";

export class ConnectionPoolConfiguration {
    public checkInterval?: number;
    public fillInterval?: number;
    public maxPoolSize?: number;

    constructor(checkInterval: number, fillInterval: number, maxPoolSize: number) {
        this.checkInterval = checkInterval;
        this.fillInterval = fillInterval;
        this.maxPoolSize = maxPoolSize;
    }
}

export class Configuration {
    public agentServerPort?: number;
    public workerThreadNumber?: number;
    public maxLogLevel?: LogLevel
    public connectionPoolConfiguration?: ConnectionPoolConfiguration

    constructor(agentServerPort: number, workerThreadNumber: number, maxLogLevel?: LogLevel, connectionPoolConfiguration?: ConnectionPoolConfiguration) {
        this.agentServerPort = agentServerPort;
        this.workerThreadNumber = workerThreadNumber;
        this.maxLogLevel = maxLogLevel;
        this.connectionPoolConfiguration = connectionPoolConfiguration;
    }
}