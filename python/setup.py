from setuptools import setup, find_packages

setup(
    name="fabric",
    version="0.1.0",
    packages=find_packages(exclude=["tests*"]),
    install_requires=[
        "eclipse-zenoh==0.11.0",
        "backoff==2.2.1",
    ],
)
