# Copyright The bngtester Authors
# Licensed under the GNU General Public License v3.0 or later.
# SPDX-License-Identifier: GPL-3.0-or-later

*** Comments ***
Tests VLAN interface creation and access method dispatch.
Containers run detached. VLAN creation is verified via entrypoint log
messages since dhcpcd may exit early in standalone Docker environments
(no DHCP server, sysctl restrictions), triggering cleanup.

*** Settings ***
Library             OperatingSystem
Library             String
Resource            ../common.robot
Resource            ../subscriber.robot

Suite Setup         Create Test Network
Suite Teardown      Destroy Test Network

*** Variables ***
${SUBSCRIBER_IMAGE}    veesixnetworks/bngtester:alpine-latest
${NET}                 bngt-vlan-test
${PREFIX}              bngt-vlan

*** Test Cases ***
Untagged Mode
    [Teardown]    Remove Container    ${PREFIX}-untagged
    ${id} =    Run Subscriber Detached    ${PREFIX}-untagged    ${SUBSCRIBER_IMAGE}
    ...    -e ENCAP=untagged -e DHCP_TIMEOUT=300    ${NET}
    Wait Until Container Logs Contain    ${PREFIX}-untagged    Target interface: eth0

Single VLAN
    [Teardown]    Remove Container    ${PREFIX}-single
    ${id} =    Run Subscriber Detached    ${PREFIX}-single    ${SUBSCRIBER_IMAGE}
    ...    -e ENCAP=single -e CVLAN=100 -e DHCP_TIMEOUT=300    ${NET}
    Wait Until Container Logs Contain    ${PREFIX}-single    Creating VLAN interface eth0.100
    Check Container Log Contains    ${PREFIX}-single    Target interface: eth0.100

QinQ VLAN
    [Teardown]    Remove Container    ${PREFIX}-qinq
    ${id} =    Run Subscriber Detached    ${PREFIX}-qinq    ${SUBSCRIBER_IMAGE}
    ...    -e ENCAP=qinq -e SVLAN=100 -e CVLAN=10 -e DHCP_TIMEOUT=300    ${NET}
    Wait Until Container Logs Contain    ${PREFIX}-qinq    Creating C-VLAN interface eth0.100.10
    Check Container Log Contains    ${PREFIX}-qinq    Creating S-VLAN interface eth0.100
    Check Container Log Contains    ${PREFIX}-qinq    Target interface: eth0.100.10

DHCPv6 Dispatch
    [Teardown]    Remove Container    ${PREFIX}-dhcpv6
    ${id} =    Run Subscriber Detached    ${PREFIX}-dhcpv6    ${SUBSCRIBER_IMAGE}
    ...    -e ACCESS_METHOD=dhcpv6 -e ENCAP=untagged -e DHCP_TIMEOUT=300    ${NET}
    Wait Until Container Logs Contain    ${PREFIX}-dhcpv6    Starting DHCPv6

PPPoE Dispatch
    [Teardown]    Remove Container    ${PREFIX}-pppoe
    ${id} =    Run Subscriber Detached    ${PREFIX}-pppoe    ${SUBSCRIBER_IMAGE}
    ...    -e ACCESS_METHOD=pppoe -e ENCAP=untagged -e PPPOE_USER=test -e PPPOE_PASSWORD=test    ${NET}
    Wait Until Container Logs Contain    ${PREFIX}-pppoe    Starting PPPoE

*** Keywords ***
Create Test Network
    Create Docker Network    ${NET}

Destroy Test Network
    FOR    ${suffix}    IN    untagged    single    qinq    dhcpv6    pppoe
        Remove Container    ${PREFIX}-${suffix}
    END
    Remove Docker Network    ${NET}

