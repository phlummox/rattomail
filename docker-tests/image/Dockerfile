
FROM ubuntu:focal

RUN apt-get clean && \
  apt-get update  && \
  DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    equivs      \
    sudo        \
  && \
  apt-get autoclean && \
  apt-get clean -y && \
  apt-get --purge -y autoremove && \
  rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*


ADD mta-dummy-control /tmp

RUN \
  mkdir -p /opt/mta && \
  cd /opt/mta && \
  equivs-build /tmp/mta-dummy-control && \
  apt install /opt/mta/mta-dummy_1.0_all.deb && \
  rm -rf /opt/mta

RUN apt-get clean && \
  apt-get update  && \
  DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    bsd-mailx \
  && \
  apt-get autoclean && \
  apt-get clean -y && \
  apt-get --purge -y autoremove && \
  rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

ARG USER_NAME=user
ARG USER_ID=1001
ARG USER_GID=1001

RUN : "adding user" && \
  set -x; \
  addgroup --gid ${USER_GID} ${USER_NAME} && \
  adduser --home /home/${USER_NAME} --disabled-password --shell /bin/bash --gid ${USER_GID} --uid ${USER_ID} --gecos '' ${USER_NAME} && \
  echo "%${USER_NAME} ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

USER ${USER_NAME}

