.PHONY: all container native clean clean-native

CONTAINER_BIN=podman
CONTAINER_FILE=container/Containerfile
CONTAINER_NAME=calcc

all: container

container: 
	${CONTAINER_BIN} build -t ${CONTAINER_NAME} -f ${CONTAINER_FILE} .

clean:
	${CONTAINER_BIN} image rm ${CONTAINER_NAME}

native:
	cargo build && \
	cargo test -- --nocapture

clean-native:
	cargo clean
