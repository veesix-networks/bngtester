# Copyright The bngtester Authors
# Licensed under the GNU General Public License v3.0 or later.
# SPDX-License-Identifier: GPL-3.0-or-later

*** Comments ***
Tests cleanup behavior on signal and failure.
Verification is log-based — once the container exits, Docker destroys
the network namespace, so we verify cleanup via log messages and exit codes.

*** Settings ***
Library             OperatingSystem
Library             String
Resource            ../common.robot
Resource            ../subscriber.robot

Suite Setup         Create Test Network
Suite Teardown      Destroy Test Network

*** Variables ***
${SUBSCRIBER_IMAGE}    veesixnetworks/bngtester:alpine-latest
${NET}                 bngt-cleanup-test
${PREFIX}              bngt-clean

*** Test Cases ***
SIGTERM Cleanup QinQ
    [Documentation]    Start QinQ subscriber, send SIGTERM, verify cleanup log messages.
    [Teardown]    Remove Container    ${PREFIX}-qinq
    ${id} =    Run Subscriber Detached    ${PREFIX}-qinq    ${SUBSCRIBER_IMAGE}
    ...    -e ENCAP=qinq -e SVLAN=100 -e CVLAN=10 -e DHCP_TIMEOUT=300    ${NET}
    Wait Until Container Logs Contain    ${PREFIX}-qinq    Creating C-VLAN interface eth0.100.10
    Send Signal If Running    ${PREFIX}-qinq    TERM
    ${exit_code} =    Wait For Container Exit    ${PREFIX}-qinq    30
    Check Container Log Contains    ${PREFIX}-qinq    Creating S-VLAN interface eth0.100

SIGTERM Cleanup Single VLAN
    [Documentation]    Start single-VLAN subscriber, send SIGTERM, verify cleanup.
    [Teardown]    Remove Container    ${PREFIX}-single
    ${id} =    Run Subscriber Detached    ${PREFIX}-single    ${SUBSCRIBER_IMAGE}
    ...    -e ENCAP=single -e CVLAN=100 -e DHCP_TIMEOUT=300    ${NET}
    Wait Until Container Logs Contain    ${PREFIX}-single    Creating VLAN interface eth0.100
    Send Signal If Running    ${PREFIX}-single    TERM
    ${exit_code} =    Wait For Container Exit    ${PREFIX}-single    30
    Check Container Log Contains    ${PREFIX}-single    Target interface: eth0.100

DHCP Timeout Exit
    [Documentation]    Start with very short DHCP timeout. Verify container exits cleanly.
    [Teardown]    Remove Container    ${PREFIX}-timeout
    ${id} =    Run Subscriber Detached    ${PREFIX}-timeout    ${SUBSCRIBER_IMAGE}
    ...    -e ENCAP=untagged -e DHCP_TIMEOUT=5    ${NET}
    ${exit_code} =    Wait For Container Exit    ${PREFIX}-timeout    30
    Check Container Log Contains    ${PREFIX}-timeout    Starting DHCPv4

*** Keywords ***
Create Test Network
    Create Docker Network    ${NET}

Destroy Test Network
    FOR    ${suffix}    IN    qinq    single    timeout
        Remove Container    ${PREFIX}-${suffix}
    END
    Remove Docker Network    ${NET}
