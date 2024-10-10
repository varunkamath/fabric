class FabricError(Exception):
    pass


class PublisherNotFoundError(FabricError):
    pass


class SubscriberNotFoundError(FabricError):
    pass


class NodeNotFoundError(FabricError):
    pass


class ConfigurationError(FabricError):
    pass


class CommunicationError(FabricError):
    pass
