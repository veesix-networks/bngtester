# Copyright The bngtester Authors
# Licensed under the GNU General Public License v3.0 or later.
# SPDX-License-Identifier: GPL-3.0-or-later

*** Comments ***
IPoE integration test through the osvbng BNG.
Uses the lab/ containerlab topology from issue #27.
Requires: containerlab, osvbng image, hugepages.
Tag: integration — exclude from CI with --exclude integration.

*** Settings ***
Library             OperatingSystem
Library             String
Library             Process
Resource            ../common.robot
Resource            ../subscriber.robot

Suite Setup         Deploy BNG Topology
Suite Teardown      Destroy BNG Topology
Force Tags          integration

*** Variables ***
${SUBSCRIBER_IMAGE}    veesixnetworks/bngtester:alpine-latest
${lab-name}            bngtester
${lab-file}            ${CURDIR}/../../lab/bngtester.clab.yml
${bng1}                clab-${lab-name}-bng1
${server}              clab-${lab-name}-server
${subscriber}          clab-${lab-name}-subscriber
${qinq-iface}          eth1.100.10

*** Test Cases ***
BNG Is Healthy
    Wait For osvbng Healthy    bng1    ${lab-name}

OSPF Adjacency Established
    Wait Until Keyword Succeeds    12 x    5s
    ...    Verify OSPF Adjacency On Router    ${server}

Subscriber QinQ Interface Created
    Wait Until Keyword Succeeds    12 x    5s
    ...    Check Interface Exists    ${subscriber}    ${qinq-iface}

Subscriber Got IPv4 Via DHCP
    Wait Until Keyword Succeeds    18 x    5s
    ...    Check Interface Has IPv4    ${subscriber}    ${qinq-iface}

Session In BNG API
    Wait Until Keyword Succeeds    30 x    2s
    ...    Check BNG Session Count    ${bng1}    1

Subscriber Can Ping Gateway
    ${rc}    ${output} =    Run And Return Rc And Output
    ...    sudo docker exec ${subscriber} ping -c 3 -W 2 10.255.0.1
    Log    ${output}
    Should Be Equal As Integers    ${rc}    0    Subscriber cannot ping gateway

Subscriber Can Ping Server Through BNG
    Wait Until Keyword Succeeds    6 x    5s
    ...    Ping From Container    ${subscriber}    10.0.0.2

Iperf3 Throughput
    [Documentation]    Informational — logs throughput but does not fail the suite.
    Start Iperf3 Server On Core    ${server}
    Wait Until Keyword Succeeds    6 x    5s
    ...    Run Iperf3 Client    ${subscriber}    10.0.0.2

*** Keywords ***
Deploy BNG Topology
    Set Environment Variable    BNGTESTER_IMAGE    ${SUBSCRIBER_IMAGE}
    Deploy Topology    ${lab-file}

Destroy BNG Topology
    Destroy Topology    ${lab-file}

Check BNG Session Count
    [Arguments]    ${container}    ${expected}
    ${output} =    Get osvbng API Response    ${container}    /api/show/subscriber/sessions
    ${rc}    ${count} =    Run And Return Rc And Output
    ...    echo '${output}' | python3 -c "import sys,json; d=json.load(sys.stdin); entries=d.get('data',[]); print(len(entries))"
    Should Be Equal As Integers    ${rc}    0
    Should Be True    ${count} >= ${expected}    Session count ${count} < expected ${expected}

Start Iperf3 Server On Core
    [Arguments]    ${container}
    ${rc}    ${output} =    Run And Return Rc And Output
    ...    sudo docker exec ${container} sh -c "iperf3 -s -D 2>/dev/null || true"
    Log    ${output}

Run Iperf3 Client
    [Arguments]    ${container}    ${server_ip}
    ${rc}    ${output} =    Run And Return Rc And Output
    ...    sudo docker exec ${container} iperf3 -c ${server_ip} -t 5
    Log    ${output}
    Should Be Equal As Integers    ${rc}    0    iperf3 failed
