class FabricError(Exception):
    pass

# Define more specific error types if needed
class ConfigurationError(FabricError):
    pass

class CommunicationError(FabricError):
    pass

class NodeError(FabricError):
    pass

class OrchestratorError(FabricError):
    pass