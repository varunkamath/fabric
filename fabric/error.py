class FabricError(Exception):
    pass

class ZenohError(FabricError):
    pass

class SerializationError(FabricError):
    pass

class PublisherNotFoundError(FabricError):
    pass

class PublishError(FabricError):
    pass

class InvalidConfigError(FabricError):
    pass