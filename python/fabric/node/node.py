import asyncio
import json
import time
import logging
import traceback
import zenoh
from .interface import NodeConfig, NodeData
from fabric.plugins import NodeRegistry

logging.basicConfig(level=logging.DEBUG)
logger = logging.getLogger(__name__)


class Node:
    def __init__(
        self, id: str, interface_type: str, config: NodeConfig, session: zenoh.Session
    ):
        self.id = id
        self.interface = NodeRegistry.create_interface(interface_type, config)
        self.session = session
        self.config = config

    @classmethod
    async def create(
        cls, id: str, interface_type: str, config: NodeConfig, session: zenoh.Session
    ):
        return cls(id, interface_type, config, session)

    async def run(self, cancel_event: asyncio.Event):
        logger.debug(f"Node {self.id} starting run method")
        try:
            publisher = await self.session.declare_publisher("node/data")
            config_subscriber = await self.session.declare_subscriber(
                f"node/{self.id}/config"
            )
            event_subscriber = await self.session.declare_subscriber(
                f"node/{self.id}/event/*"
            )

            while not cancel_event.is_set():
                logger.debug(f"Node {self.id} running main loop")
                try:
                    value = await self.interface.read()
                    logger.debug(f"Node {self.id} read value: {value}")
                    data = NodeData(
                        node_id=self.id,
                        interface_type=self.interface.get_type(),
                        value=value,
                        timestamp=int(time.time()),
                        metadata=None,
                    )
                    json_data = json.dumps(data.to_dict())
                    logger.debug(f"Node {self.id} publishing data: {json_data}")
                    put_result = publisher.put(json_data)
                    if asyncio.iscoroutine(put_result):
                        await put_result
                except Exception as e:
                    logger.error(
                        f"Error reading or publishing data for node {self.id}: {str(e)}"
                    )
                    logger.error(traceback.format_exc())

                try:
                    config_sample = await asyncio.wait_for(
                        config_subscriber.recv(), timeout=0.1
                    )
                    if config_sample:
                        payload = config_sample.payload
                        if asyncio.iscoroutine(payload):
                            payload = await payload
                        if isinstance(payload, bytes):
                            payload = payload.decode()
                        new_config = NodeConfig(**json.loads(payload))
                        self.interface.set_config(new_config)
                        self.config = new_config  # Update the node's config as well
                        logger.debug(f"Node {self.id} updated config: {new_config}")
                except asyncio.TimeoutError:
                    pass
                except json.JSONDecodeError as e:
                    logger.error(f"Error decoding config for node {self.id}: {str(e)}")
                    logger.error(traceback.format_exc())
                except Exception as e:
                    logger.error(
                        f"Error processing config for node {self.id}: {str(e)}"
                    )
                    logger.error(traceback.format_exc())

                try:
                    event_sample = await asyncio.wait_for(
                        event_subscriber.recv(), timeout=0.1
                    )
                    if event_sample:
                        key_expr = event_sample.key_expr
                        if asyncio.iscoroutine(key_expr):
                            key_expr = await key_expr
                        event = key_expr.as_string().split("/")[-1]
                        payload = event_sample.payload
                        if asyncio.iscoroutine(payload):
                            payload = await payload
                        if isinstance(payload, bytes):
                            payload = payload.decode()
                        logger.debug(
                            f"Node {self.id} received event: {event}, payload: {payload}"
                        )
                        await self.handle_event(event, payload)
                        break  # Exit the loop after handling the event
                except asyncio.TimeoutError:
                    pass
                except Exception as e:
                    logger.error(f"Error processing event in node {self.id}: {str(e)}")
                    logger.error(traceback.format_exc())

                await asyncio.sleep(1 / self.config.sampling_rate)

        except KeyboardInterrupt:
            logger.info(f"Node {self.id} received KeyboardInterrupt, shutting down")
        except Exception as e:
            logger.error(f"Error in node {self.id}: {str(e)}")
            logger.error(traceback.format_exc())
        finally:
            logger.debug(f"Node {self.id} exiting run method")

    def get_config(self) -> NodeConfig:
        return self.config

    async def handle_event(self, event: str, payload: str):
        logger.debug(f"Node {self.id} handling event: {event}, payload: {payload}")
        await self.interface.handle_event(event, payload)
