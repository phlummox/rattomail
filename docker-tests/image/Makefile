
default:
	echo pass

NAME=phlummox/test-rattomail
TAG=0.1

build:
	docker build -t  $(NAME):$(TAG) .

run:
	docker -D run -e DISPLAY -it --rm  --net=host  \
			-v $(PWD):/work \
			--workdir /work \
	    $(MOUNT)     \
	    $(NAME):$(TAG) bash

