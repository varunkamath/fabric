from setuptools import setup, find_packages

setup(
    name="fabric",
    version="0.1.0",
    packages=find_packages(),
    install_requires=[
        "zenoh",
        "pytest",
        "pytest-asyncio",
    ],
)